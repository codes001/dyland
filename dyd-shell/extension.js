/* GNOME Shell integration for the dyland overlay.
 *
 * Mutter does not implement zwlr_layer_shell_v1, and a normal client window
 * cannot stack above the top bar: the panel lives in the shell UI group,
 * above every window group including "always on top". This extension is the
 * supported integration point. It hides the center clock and moves the
 * dyland surface into that UI group, on the primary monitor.
 *
 * The island itself is still the Tauri window (role "dyland-overlay").
 * Mutter treats it as a dock: always on every workspace, and absent from
 * the dash. A normal window's stick() is cleared by any workspace move.
 */

import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';

const ROLE = 'dyland-overlay';
const APP_ID = 'app.dyland';
const CALL_APPS = [
    'whatsapp', 'snapchat', 'telegram', 'signal', 'discord', 'slack',
    'zoom', 'skype', 'viber', 'teams', 'messenger', 'caprine', 'element',
    'wechat', 'webex',
];
const CALL_TEXT = /incoming (?:voice |video )?call|voice call|video call|\bis calling\b|calling you|\bringing\b|webrtc|ongoing call|on a call/i;

function looksLikeCall(app, title) {
    const blob = `${app || ''} ${title || ''}`;
    if (CALL_TEXT.test(blob))
        return true;
    const lowerApp = (app || '').toLowerCase();
    const lowerTitle = (title || '').trim().toLowerCase();
    const known = CALL_APPS.some(name => lowerApp.includes(name) || lowerTitle.includes(name));
    return known && /^(?:incoming )?(?:voice |video )?call$/.test(lowerTitle);
}

export default class DylandExtension extends Extension {
    enable() {
        this._window = null;
        this._actor = null;
        this._windowSignals = [];
        this._clock = null;
        this._clockWasVisible = false;
        this._clockHidden = false;
        this._overview = false;
        this._loggedAdopt = false;
        this._loggedMiss = false;
        this._loggedChrome = false;
        this._enabled = true;
        this._raising = false;
        this._chroming = false;
        this._keepingShown = false;
        this._actorSignals = [];
        this._callKey = '';

        const uiGroup = this._uiGroup();
        this._displayId = global.display.connect('window-created', (_display, window) => {
            if (this._isIsland(window))
                this._adopt(window);
        });
        this._monitorsId = Main.layoutManager.connect('monitors-changed', () => {
            this._ensureRaised();
        });
        this._sessionId = Main.sessionMode.connect('updated', () => {
            this._ensureRaised();
        });
        this._overviewShowingId = Main.overview.connect('showing', () => {
            this._overview = true;
            this._park();
        });
        this._overviewHiddenId = Main.overview.connect('hidden', () => {
            this._overview = false;
            this._ensureRaised();
        });
        this._workspaceId = global.workspace_manager?.connect('workspace-switched', () => {
            this._chrome();
            this._ensureRaised();
        });
        this._watchId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 1000, () => {
            if (!this._window)
                this._scanExisting();
            this._chrome();
            this._ensureRaised();
            this._scanCalls();
            return GLib.SOURCE_CONTINUE;
        });

        this._scanExisting();
        if (!uiGroup)
            console.log('dyland: shell UI group is missing; overlay cannot cover the top bar');
    }

    disable() {
        this._enabled = false;
        this._disconnect(global.display, this._displayId);
        this._disconnect(Main.layoutManager, this._monitorsId);
        this._disconnect(Main.sessionMode, this._sessionId);
        this._disconnect(Main.overview, this._overviewShowingId);
        this._disconnect(Main.overview, this._overviewHiddenId);
        this._disconnect(global.workspace_manager, this._workspaceId);
        if (this._watchId) {
            GLib.source_remove(this._watchId);
            this._watchId = 0;
        }
        this._park();
        this._releaseWindow();
        this._showClock();
        this._window = null;
        this._actor = null;
    }

    _scanExisting() {
        const actors = global.get_window_actors?.() ?? [];
        for (const actor of actors) {
            const meta = actor.meta_window || actor.metaWindow;
            if (this._isIsland(meta)) {
                this._adopt(meta);
                return;
            }
        }
        const windows = global.display?.list_all_windows?.() ?? [];
        for (const meta of windows) {
            if (this._isIsland(meta)) {
                this._adopt(meta);
                return;
            }
        }
        if (!this._loggedMiss) {
            this._loggedMiss = true;
            console.log('dyland: island window is not on screen yet');
        }
    }

    _isIsland(metaWindow) {
        if (!metaWindow)
            return false;
        try {
            if (metaWindow.get_role?.() === ROLE)
                return true;
        } catch (e) {
            // Role is not available for every surface. Fall through.
        }
        const appId = metaWindow.get_gtk_application_id?.();
        if (appId === APP_ID)
            return true;
        const wmClass = metaWindow.get_wm_class?.() ?? '';
        if (wmClass === 'dyland' || wmClass === APP_ID)
            return true;
        return metaWindow.get_title?.() === 'dyland';
    }

    _adopt(metaWindow) {
        if (!this._enabled || !metaWindow || metaWindow === this._window)
            return;
        this._releaseWindow();
        const actor = metaWindow.get_compositor_private?.();
        if (!actor) {
            let tries = 0;
            GLib.idle_add(GLib.PRIORITY_DEFAULT, () => {
                tries += 1;
                if (!this._enabled || tries > 50 || !this._isIsland(metaWindow))
                    return GLib.SOURCE_REMOVE;
                if (!metaWindow.get_compositor_private?.())
                    return GLib.SOURCE_CONTINUE;
                this._adopt(metaWindow);
                return GLib.SOURCE_REMOVE;
            });
            return;
        }
        this._window = metaWindow;
        this._actor = actor;
        this._loggedChrome = false;
        this._windowSignals = [
            metaWindow.connect('position-changed', () => this._ensureRaised()),
            metaWindow.connect('size-changed', () => this._ensureRaised()),
            metaWindow.connect('workspace-changed', () => {
                this._chrome();
                this._ensureRaised();
            }),
            metaWindow.connect('notify::window-type', () => this._chrome()),
            metaWindow.connect('notify::on-all-workspaces', () => this._chrome()),
            metaWindow.connect('unmanaged', () => {
                this._releaseWindow();
                this._showClock();
            }),
        ];
        this._actorSignals = [
            actor.connect('notify::x', () => this._ensureRaised()),
            actor.connect('notify::y', () => this._ensureRaised()),
            actor.connect('notify::visible', () => this._keepShown()),
        ];
        this._chrome();
        this._loggedAdopt = false;
        this._ensureRaised();
    }

    // Wayland ignores the client's stick() and skip-taskbar hints, so the
    // binary shows up on the dock and rides with one workspace. A dock window
    // is always sticky: a later workspace move cannot clear that. Moving a
    // stick()-only window does clear it, so this never calls change_workspace.
    _chrome() {
        const metaWindow = this._window;
        if (!metaWindow || this._chroming)
            return;
        this._chroming = true;
        try {
            const dock = Meta.WindowType?.DOCK;
            let type = null;
            try {
                type = metaWindow.get_window_type?.();
            } catch (e) {
                console.log(`dyland: window type unavailable: ${e}`);
            }
            if (dock !== undefined && type !== dock) {
                try {
                    metaWindow.set_type(dock);
                } catch (e) {
                    console.log(`dyland: could not mark the island as shell chrome: ${e}`);
                }
            }
            try {
                if (!metaWindow.is_on_all_workspaces())
                    metaWindow.stick();
            } catch (e) {
                console.log(`dyland: stick failed: ${e}`);
            }
            try {
                metaWindow.make_above();
            } catch (e) {
                // Keep-above is also set by the client window.
            }
            if (!this._loggedChrome) {
                this._loggedChrome = true;
                let after = type;
                let all = false;
                let always = false;
                let skip = false;
                try {
                    after = metaWindow.get_window_type();
                    all = metaWindow.is_on_all_workspaces();
                    always = metaWindow.is_always_on_all_workspaces();
                    skip = metaWindow.is_skip_taskbar();
                } catch (e) {
                    console.log(`dyland: chrome state unavailable: ${e}`);
                }
                const docked = after === dock;
                console.log(`dyland: chrome dock=${docked} type=${after} all=${all} always=${always} skip=${skip}`);
            }
            this._keepShown();
        } finally {
            this._chroming = false;
        }
    }

    _keepShown() {
        if (this._overview || this._sessionLocked() || this._keepingShown)
            return;
        const actor = this._actor;
        if (!actor)
            return;
        this._keepingShown = true;
        try {
            if (!actor.visible)
                actor.show();
            actor.opacity = 255;
        } catch (e) {
            // The actor can already be gone during unmanage.
        } finally {
            this._keepingShown = false;
        }
    }

    _scanCalls() {
        let hit = null;
        const windows = global.display?.list_all_windows?.() ?? [];
        for (const meta of windows) {
            if (!meta || this._isIsland(meta))
                continue;
            let title = '';
            let wmClass = '';
            let appId = '';
            try {
                title = meta.get_title?.() || '';
                wmClass = meta.get_wm_class?.() || '';
                appId = meta.get_gtk_application_id?.() || '';
            } catch (e) {
                continue;
            }
            const app = `${wmClass} ${appId}`.trim();
            if (!looksLikeCall(app, title))
                continue;
            const appName = wmClass || appId || 'Call';
            hit = { appName, summary: title || 'Incoming call' };
            break;
        }
        const key = hit ? `${hit.appName}|${hit.summary}` : 'off';
        if (key === this._callKey)
            return;
        this._callKey = key;
        this._reportCall(Boolean(hit), hit?.appName ?? '', hit?.summary ?? '');
    }

    _reportCall(active, appName, summary) {
        try {
            Gio.DBus.session.call(
                'app.dyland',
                '/app/dyland/Island',
                'app.dyland.Island',
                'SetIncomingCall',
                new GLib.Variant('(ssb)', [appName, summary, active]),
                null,
                Gio.DBusCallFlags.NO_AUTO_START,
                -1,
                null,
                null,
            );
        } catch (e) {
            console.log(`dyland: call report failed: ${e}`);
        }
    }

    _ensureRaised() {
        if (this._raising || !this._enabled || !this._window || !this._actor)
            return;
        this._raising = true;
        try {
            this._raiseNow();
        } finally {
            this._raising = false;
        }
    }

    _raiseNow() {
        if (this._overview || this._sessionLocked()) {
            this._park();
            this._showClock();
            return;
        }
        this._chrome();
        this._stackAbovePanel();
        this._hideClock();
        this._place();
    }

    _stackAbovePanel() {
        const uiGroup = this._uiGroup();
        const actor = this._actor;
        if (!uiGroup || !actor)
            return;
        // window_group lives under the top bar. The clock is chrome in uiGroup,
        // so the island actor has to move there or Mutter keeps it centered
        // below the panel.
        try {
            const parent = actor.get_parent?.();
            if (parent !== uiGroup) {
                parent?.remove_child(actor);
                uiGroup.add_child(actor);
            }
            const panel = Main.layoutManager.panelBox;
            if (panel)
                uiGroup.set_child_above_sibling(actor, panel);
        } catch (e) {
            console.log(`dyland: failed to stack overlay: ${e}`);
        }
    }

    _place() {
        const monitor = Main.layoutManager.primaryMonitor;
        if (!monitor || !this._window)
            return;
        const frame = this._window.get_frame_rect();
        const width = frame.width || 440;
        const height = frame.height || 96;
        const x = monitor.x + Math.round((monitor.width - width) / 2);
        const y = monitor.y;
        if (Math.abs(frame.x - x) > 1 || Math.abs(frame.y - y) > 1) {
            try {
                this._window.move_resize_frame(false, x, y, width, height);
            } catch (e) {
                console.log(`dyland: move failed: ${e}`);
            }
        }
        const actor = this._actor;
        if (actor && (Math.abs(actor.x - x) > 1 || Math.abs(actor.y - y) > 1))
            actor.set_position(x, y);
        if (!this._loggedAdopt) {
            this._loggedAdopt = true;
            console.log(`dyland: overlay above the top bar at ${x},${y} on ${monitor.width}x${monitor.height}`);
        }
    }

    _park() {
        const actor = this._actor;
        const group = global.window_group;
        if (!actor || !group)
            return;
        try {
            const parent = actor.get_parent?.();
            if (parent && parent !== group) {
                parent.remove_child(actor);
                group.add_child(actor);
            }
        } catch (e) {
            console.log(`dyland: failed to return overlay to the window group: ${e}`);
        }
    }

    _sessionLocked() {
        const mode = Main.sessionMode.currentMode;
        return mode === 'lock' || mode === 'unlock-dialog' || mode === 'greeter';
    }

    _hideClock() {
        const clock = Main.panel.statusArea.dateMenu;
        if (!clock)
            return;
        if (!this._clock) {
            this._clock = clock;
            this._clockWasVisible = clock.visible !== false;
        }
        if (clock.visible !== false) {
            if (typeof clock.hide === 'function')
                clock.hide();
            else if (clock.container)
                clock.container.hide();
        }
        this._clockHidden = true;
    }

    _showClock() {
        if (!this._clockHidden || !this._clock)
            return;
        if (this._clockWasVisible) {
            if (typeof this._clock.show === 'function')
                this._clock.show();
            else if (this._clock.container)
                this._clock.container.show();
        }
        this._clockHidden = false;
    }

    _releaseWindow() {
        if (this._window && this._windowSignals) {
            for (const id of this._windowSignals)
                this._window.disconnect(id);
        }
        if (this._actor && this._actorSignals) {
            for (const id of this._actorSignals) {
                try {
                    this._actor.disconnect(id);
                } catch (e) {
                    // The actor may already be destroyed with the window.
                }
            }
        }
        this._windowSignals = [];
        this._actorSignals = [];
        this._window = null;
        this._actor = null;
    }

    _uiGroup() {
        return Main.layoutManager.uiGroup || Main.uiGroup;
    }

    _disconnect(object, id) {
        if (object && id)
            object.disconnect(id);
    }
}
