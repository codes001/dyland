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

import Atspi from 'gi://Atspi';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import { MessageTray } from 'resource:///org/gnome/shell/ui/messageTray.js';

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

function isCallBanner(notification) {
    if (!notification)
        return false;
    const title = notification.title || '';
    const body = notification.body || notification.bannerBodyText || '';
    const source = notification.source?.title || '';
    const labels = (notification.actions || []).map(action => action?.label || '').join(' ');
    if (looksLikeCall(source, `${title} ${body}`))
        return true;
    const blob = `${source} ${title} ${body} ${labels}`.toLowerCase();
    const answers = ['answer', 'accept', 'decline', 'reject'].some(word => blob.includes(word));
    return answers && blob.includes('call');
}

const ACCEPT_RE = /\b(answer|accept|pick[\s-]*up|join call|take call)\b/;
const REJECT_RE = /\b(decline|reject|hang[\s-]*up|ignore|end(?:\s+call)?|leave(?:\s+call)?|deny|cancel call)\b/;
const RING_TITLE = /incoming (?:voice |video )?call|\bis calling\b|calling you|\bringing\b/i;
const ACTIVE_TITLE = /ongoing call|on a call|(?:voice|video) call with|\bcall with\b|in a call/i;
const CALL_APP_RE = /whatsapp|snapchat|discord|telegram|signal|slack|zoom|skype|viber|teams|messenger|caprine|element|wechat|webex|threema|firefox|chrome|chromium|brave|opera|edge|epiphany|gnome-calls|\bcalls\b|\bphone\b|dialer/;

function actionLabel(action) {
    try {
        if (typeof action?.label === 'string' && action.label)
            return action.label;
        if (typeof action?.get_label === 'function')
            return action.get_label() || '';
    } catch (e) {
        // A disposed notification action has no label.
    }
    return '';
}

function pickCallAction(notification, kind) {
    const actions = notification?.actions || [];
    const labeled = actions.map(action => ({ action, label: actionLabel(action).toLowerCase() }));
    const wanted = kind === 'accept' ? ACCEPT_RE : REJECT_RE;
    const other = kind === 'accept' ? REJECT_RE : ACCEPT_RE;
    const direct = labeled.find(item => wanted.test(item.label));
    if (direct)
        return direct.action;
    const rest = labeled.filter(item => item.label && !other.test(item.label));
    if (labeled.some(item => other.test(item.label)) && rest.length === 1)
        return rest[0].action;
    return null;
}

function accessibleText(node) {
    let name = '';
    let description = '';
    try {
        name = node.get_name() || '';
    } catch (e) {
        name = '';
    }
    try {
        description = node.get_description() || '';
    } catch (e) {
        description = '';
    }
    let actions = '';
    try {
        const count = node.get_n_actions();
        for (let i = 0; i < count && i < 4; i++)
            actions += ` ${node.get_action_name(i) || ''} ${node.get_action_description(i) || ''}`;
    } catch (e) {
        actions = '';
    }
    return `${name} ${description} ${actions}`.toLowerCase();
}

function attributeBlob(node) {
    try {
        const attrs = node.get_attributes();
        if (!attrs || typeof attrs.length !== 'number')
            return '';
        let text = '';
        for (let i = 0; i < attrs.length && i < 32; i++)
            text += ` ${attrs[i]}`;
        return text;
    } catch (e) {
        return '';
    }
}

function skipCallBranch(role, count) {
    if (count <= 8)
        return false;
    return role === Atspi.Role.LIST
        || role === Atspi.Role.TABLE
        || role === Atspi.Role.TREE
        || role === Atspi.Role.TREE_TABLE
        || role === Atspi.Role.LIST_BOX;
}

function pressAccessible(node) {
    let count = 0;
    try {
        count = node.get_n_actions();
    } catch (e) {
        return false;
    }
    if (count < 1)
        return false;
    let index = 0;
    for (let i = 0; i < count; i++) {
        let name = '';
        try {
            name = (node.get_action_name(i) || '').toLowerCase();
        } catch (e) {
            name = '';
        }
        if (name.includes('click') || name === 'press' || name === 'activate') {
            index = i;
            break;
        }
    }
    try {
        return Boolean(node.do_action(index));
    } catch (e) {
        return false;
    }
}

function ancestorContext(node) {
    let text = '';
    let current = node;
    for (let i = 0; i < 8 && current; i++) {
        text += ` ${accessibleText(current)}`;
        try {
            current = current.get_parent();
        } catch (e) {
            break;
        }
    }
    return text;
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
        this._callWindow = null;
        this._callNotification = null;
        this._acceptAction = null;
        this._rejectAction = null;
        this._answered = false;
        this._callWatch = 0;
        this._sawCallControl = false;
        this._missedCallControl = 0;
        this._controlSub = 0;
        this._bannerProto = null;
        this._showNotification = null;
        this._sourceWatches = [];
        this._traySourceId = 0;

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
            this._suppressVisibleCallBanner();
            this._captureVisibleCallActions();
            return GLib.SOURCE_CONTINUE;
        });

        this._scanExisting();
        this._installCallControls();
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
        this._removeCallControls();
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
            const ringing = RING_TITLE.test(title);
            const active = !ringing && ACTIVE_TITLE.test(title);
            if (!ringing && !active && !looksLikeCall(app, title))
                continue;
            const appName = wmClass || appId || 'Call';
            const phase = active ? 'active' : 'ringing';
            hit = {
                appName,
                summary: title || (phase === 'active' ? 'On call' : 'Incoming call'),
                meta,
                phase,
            };
            break;
        }
        this._callWindow = hit?.meta || null;
        const key = hit ? `${hit.phase}|${hit.appName}|${hit.summary}` : 'off';
        if (key !== this._callKey && hit && (this._callKey === '' || this._callKey === 'off')) {
            this._acceptAction = null;
            this._rejectAction = null;
        }
        const same = key === this._callKey;
        this._callKey = key;
        if (!same)
            this._reportCall(Boolean(hit), hit?.appName ?? '', hit?.summary ?? '');
        if (hit?.phase === 'active' && !this._answered)
            this._markAnswered(this._callNotification);
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

    _installCallControls() {
        try {
            const proto = MessageTray.prototype;
            if (typeof proto._showNotification !== 'function') {
                console.log('dyland: message tray has no _showNotification');
            } else {
                this._bannerProto = proto;
                this._showNotification = proto._showNotification;
                const extension = this;
                proto._showNotification = function () {
                    while (this._notificationQueue?.[0] && isCallBanner(this._notificationQueue[0])) {
                        const skipped = this._notificationQueue.shift();
                        extension._watchNotification(skipped);
                        extension._captureCallActions(skipped);
                        skipped.acknowledged = true;
                        this.emit('queue-changed');
                    }
                    if (!this._notificationQueue?.[0] || typeof extension._showNotification !== 'function')
                        return undefined;
                    return extension._showNotification.apply(this, arguments);
                };
            }
        } catch (e) {
            console.log(`dyland: call banner filter unavailable: ${e}`);
        }
        try {
            this._controlSub = Gio.DBus.session.signal_subscribe(
                null,
                'app.dyland.Island',
                'CallControl',
                '/app/dyland/Island',
                null,
                Gio.DBusSignalFlags.NONE,
                (_conn, _sender, _path, _iface, _signal, parameters) => {
                    let action = parameters?.deepUnpack?.() ?? '';
                    if (Array.isArray(action))
                        action = action[0];
                    this._onCallControl(String(action || ''));
                },
            );
        } catch (e) {
            console.log(`dyland: call control listener failed: ${e}`);
        }
        this._watchNotificationSources();
        this._suppressVisibleCallBanner();
    }

    _removeCallControls() {
        if (this._bannerProto && this._showNotification)
            this._bannerProto._showNotification = this._showNotification;
        this._bannerProto = null;
        this._showNotification = null;
        if (this._controlSub) {
            try {
                Gio.DBus.session.signal_unsubscribe(this._controlSub);
            } catch (e) {
                console.log(`dyland: call control unsubscribe failed: ${e}`);
            }
            this._controlSub = 0;
        }
        if (this._traySourceId)
            this._disconnect(Main.messageTray, this._traySourceId);
        this._traySourceId = 0;
        for (const watch of this._sourceWatches || []) {
            try {
                watch[0].disconnect(watch[1]);
                watch[0]._dylandTracked = false;
                watch[0]._dylandWatched = false;
            } catch (e) {
                // The source may already be destroyed with the notification.
            }
        }
        this._sourceWatches = [];
        this._acceptAction = null;
        this._rejectAction = null;
        this._callNotification = null;
        this._callWindow = null;
        this._answered = false;
        if (this._callWatch) {
            GLib.source_remove(this._callWatch);
            this._callWatch = 0;
        }
    }

    _holdCall(notification) {
        if (!notification)
            return;
        this._callNotification = notification;
        if (notification._dylandHeld)
            return;
        notification._dylandHeld = true;
        if (!notification._dylandBannerLogged) {
            notification._dylandBannerLogged = true;
            const title = notification.title || notification.source?.title || 'call';
            console.log(`dyland: hid the GNOME call banner for ${title}`);
        }
        try {
            notification.connect('destroy', () => {
                if (this._callNotification === notification)
                    this._callNotification = null;
            });
        } catch (e) {
            // The notification can already be going away.
        }
    }

    _suppressVisibleCallBanner() {
        const tray = Main.messageTray;
        if (!tray)
            return;
        if (Array.isArray(tray._notificationQueue)) {
            const kept = [];
            let dropped = false;
            for (const queued of tray._notificationQueue) {
                if (isCallBanner(queued)) {
                    this._holdCall(queued);
                    queued.acknowledged = true;
                    dropped = true;
                } else {
                    kept.push(queued);
                }
            }
            if (dropped)
                tray._notificationQueue = kept;
        }
        const notification = tray._notification;
        if (!notification || !isCallBanner(notification))
            return;
        const showing = tray._notificationState === 1 || tray._notificationState === 2;
        if (!showing)
            return;
        this._holdCall(notification);
        notification.acknowledged = true;
        tray._notificationRemoved = true;
        try {
            tray._updateState();
        } catch (e) {
            console.log(`dyland: failed to hide the call banner: ${e}`);
        }
    }

    _onCallControl(action) {
        const name = String(action || '').toLowerCase();
        if (name !== 'accept' && name !== 'reject' && name !== 'decline')
            return;
        const kind = name === 'accept' ? 'accept' : 'reject';
        const now = GLib.get_monotonic_time();
        if (now - (this._callControlAt || 0) < 700000)
            return;
        this._callControlAt = now;

        this._captureVisibleCallActions();
        const notification = this._bestCallNotification();
        const labels = (notification?.actions || []).map(actionLabel).filter(Boolean);
        const button = this._liveCallAction(kind) || (kind === 'accept' ? this._acceptAction : this._rejectAction);
        const saved = button ? actionLabel(button) : 'none';
        console.log(`dyland: ${kind} requested; banner actions=${labels.join(', ') || 'none'}; using=${saved}`);
        if (button) {
            try {
                button.activate();
                console.log(`dyland: ${kind} used the notification button "${actionLabel(button)}"`);
                if (kind === 'accept')
                    this._markAnswered(notification);
                else
                    this._reportEnded('notification');
                return;
            } catch (e) {
                console.log(`dyland: notification ${kind} failed: ${e}`);
            }
        }

        const meta = this._findCallWindow();
        if (meta)
            this._activateWindow(meta);
        if (this._pressAccessibleCallButton(kind))
            return;
        GLib.timeout_add(GLib.PRIORITY_DEFAULT, 400, () => {
            if (!this._enabled)
                return GLib.SOURCE_REMOVE;
            if (this._pressAccessibleCallButton(kind))
                return GLib.SOURCE_REMOVE;
            GLib.timeout_add(GLib.PRIORITY_DEFAULT, 700, () => {
                if (!this._enabled)
                    return GLib.SOURCE_REMOVE;
                if (!this._pressAccessibleCallButton(kind))
                    console.log(`dyland: no ${kind} button in the call app`);
                return GLib.SOURCE_REMOVE;
            });
            return GLib.SOURCE_REMOVE;
        });
    }

    _markAnswered(notification) {
        const app = notification?.source?.title || 'Call';
        const summary = notification?.title || 'Call';
        this._answered = true;
        this._sawCallControl = false;
        this._missedCallControl = 0;
        this._callDaemon('SetCallAnswered', new GLib.Variant('(ss)', [String(app), String(summary)]));
        if (this._callWatch)
            GLib.source_remove(this._callWatch);
        this._callWatch = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 2000, () => {
            if (!this._enabled || !this._answered)
                return GLib.SOURCE_REMOVE;
            if (this._findCallButton('reject')) {
                this._sawCallControl = true;
                this._missedCallControl = 0;
                return GLib.SOURCE_CONTINUE;
            }
            if (!this._sawCallControl)
                return GLib.SOURCE_CONTINUE;
            this._missedCallControl += 1;
            if (this._missedCallControl < 3)
                return GLib.SOURCE_CONTINUE;
            this._reportEnded('controls-gone');
            return GLib.SOURCE_REMOVE;
        });
    }

    _reportEnded(reason) {
        this._answered = false;
        this._sawCallControl = false;
        this._missedCallControl = 0;
        if (this._callWatch) {
            GLib.source_remove(this._callWatch);
            this._callWatch = 0;
        }
        this._callDaemon('SetCallEnded', new GLib.Variant('(s)', [String(reason || 'ended')]));
    }

    _callDaemon(method, parameters) {
        try {
            Gio.DBus.session.call(
                'app.dyland',
                '/app/dyland/Island',
                'app.dyland.Island',
                method,
                parameters,
                null,
                Gio.DBusCallFlags.NO_AUTO_START,
                -1,
                null,
                (_connection, result) => {
                    try {
                        Gio.DBus.session.call_finish(result);
                    } catch (e) {
                        console.log(`dyland: ${method} failed: ${e}`);
                    }
                },
            );
        } catch (e) {
            console.log(`dyland: ${method} failed: ${e}`);
        }
    }

    _watchNotificationSources() {
        const tray = Main.messageTray;
        if (!tray)
            return;
        try {
            this._traySourceId = tray.connect('source-added', (_tray, source) => {
                this._trackSource(source);
            });
        } catch (e) {
            console.log(`dyland: could not watch notification sources: ${e}`);
        }
        for (const source of tray.getSources?.() || [])
            this._trackSource(source);
    }

    _trackSource(source) {
        if (!source || source._dylandTracked)
            return;
        source._dylandTracked = true;
        try {
            const added = source.connect('notification-added', (_source, notification) => {
                this._watchNotification(notification);
            });
            this._sourceWatches.push([source, added]);
        } catch (e) {
            console.log(`dyland: could not watch a notification source: ${e}`);
        }
        for (const notification of source.notifications || [])
            this._watchNotification(notification);
    }

    _watchNotification(notification) {
        if (!notification || notification._dylandWatched)
            return;
        notification._dylandWatched = true;
        const capture = () => this._captureCallActions(notification);
        try {
            const id = notification.connect('action-added', () => capture());
            this._sourceWatches.push([notification, id]);
        } catch (e) {
            // The notification can already be going away.
        }
        capture();
    }

    _captureCallActions(notification) {
        if (!notification)
            return;
        let accept = null;
        let reject = null;
        for (const action of notification.actions || []) {
            const label = actionLabel(action).toLowerCase();
            if (!label || label === 'settings')
                continue;
            if (ACCEPT_RE.test(label))
                accept = action;
            else if (REJECT_RE.test(label))
                reject = action;
        }
        if (accept)
            this._acceptAction = accept;
        if (reject)
            this._rejectAction = reject;
        if ((accept || reject) && !notification._dylandActionsLogged) {
            notification._dylandActionsLogged = true;
            const accepted = accept ? actionLabel(accept) : 'none';
            const rejected = reject ? actionLabel(reject) : 'none';
            console.log(`dyland: saved call actions accept=${accepted} decline=${rejected}`);
        }
        if (accept || reject || isCallBanner(notification))
            this._holdCall(notification);
    }

    _captureVisibleCallActions() {
        const sources = Main.messageTray?.getSources?.() || [];
        for (const source of sources) {
            for (const notification of source.notifications || [])
                this._captureCallActions(notification);
        }
        this._captureCallActions(this._callNotification);
    }

    _liveCallAction(kind) {
        const notifications = [];
        if (this._callNotification)
            notifications.push(this._callNotification);
        const sources = Main.messageTray?.getSources?.() || [];
        for (const source of sources) {
            for (const notification of source.notifications || [])
                notifications.push(notification);
        }
        const seen = new Set();
        for (const notification of notifications) {
            if (!notification || seen.has(notification))
                continue;
            seen.add(notification);
            const button = pickCallAction(notification, kind);
            if (button)
                return button;
        }
        return null;
    }

    _bestCallNotification() {
        let fallback = this._callNotification;
        const sources = Main.messageTray?.getSources?.() || [];
        for (const source of sources) {
            for (const notification of source.notifications || []) {
                const title = notification.title || '';
                const body = notification.body || '';
                const sourceTitle = source.title || notification.source?.title || '';
                if (!isCallBanner(notification) && !looksLikeCall(sourceTitle, `${title} ${body}`))
                    continue;
                if ((notification.actions || []).length)
                    return notification;
                fallback = notification;
            }
        }
        return fallback;
    }

    _findCallWindow() {
        if (this._callWindow)
            return this._callWindow;
        let known = null;
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
            const app = `${wmClass} ${appId}`;
            if (looksLikeCall(app, title))
                return meta;
            const blob = `${app} ${title}`.toLowerCase();
            if (CALL_APP_RE.test(blob))
                known = known || meta;
        }
        return known;
    }

    _activateWindow(meta) {
        try {
            const time = global.get_current_time?.() || 0;
            meta.get_workspace?.()?.activate?.(time);
            meta.activate(time);
            let title = '';
            try {
                title = meta.get_title?.() || meta.get_wm_class?.() || '';
            } catch (e) {
                title = '';
            }
            console.log(`dyland: focused call window ${title}`);
        } catch (e) {
            console.log(`dyland: focus call failed: ${e}`);
        }
    }

    _atspiDesktop() {
        try {
            if (!this._atspiReady) {
                Atspi.init();
                this._atspiReady = true;
            }
            return Atspi.get_desktop(0);
        } catch (e) {
            console.log(`dyland: accessibility is unavailable: ${e}`);
            return null;
        }
    }

    _findCallButton(kind) {
        const desktop = this._atspiDesktop();
        if (!desktop)
            return null;
        const wanted = kind === 'accept' ? ACCEPT_RE : REJECT_RE;
        const frames = this._callFrames(desktop);
        const roots = frames.length ? frames : this._foreignApps(desktop);
        let best = null;
        for (const root of roots) {
            const hits = [];
            this._collectCallButtons(root, wanted, kind, 0, { seen: 0 }, hits);
            for (const hit of hits) {
                if (!best || hit.score > best.score)
                    best = hit;
            }
            if (best && frames.length)
                break;
        }
        return best;
    }

    _pressAccessibleCallButton(kind) {
        const found = this._findCallButton(kind);
        if (!found)
            return false;
        try {
            if (!pressAccessible(found.node))
                return false;
            const label = accessibleText(found.node).trim();
            console.log(`dyland: ${kind} pressed "${label || 'button'}"`);
            if (kind === 'accept')
                this._markAnswered(this._callNotification);
            else
                this._confirmEnded();
            return true;
        } catch (e) {
            console.log(`dyland: accessibility ${kind} failed: ${e}`);
            return false;
        }
    }

    _confirmEnded() {
        GLib.timeout_add(GLib.PRIORITY_DEFAULT, 1200, () => {
            if (!this._enabled)
                return GLib.SOURCE_REMOVE;
            if (!this._findCallButton('reject'))
                this._reportEnded('button');
            return GLib.SOURCE_REMOVE;
        });
    }

    _callFrames(desktop) {
        const frames = [];
        let count = 0;
        try {
            count = desktop.get_child_count();
        } catch (e) {
            return frames;
        }
        for (let i = 0; i < count; i++) {
            let app = null;
            try {
                app = desktop.get_child_at_index(i);
            } catch (e) {
                continue;
            }
            if (!app)
                continue;
            const appName = accessibleText(app);
            if (/dyland|dyd-ui|gnome-shell|ibus/.test(appName))
                continue;
            const browser = /firefox|chrome|chromium|brave|opera|edge|epiphany/.test(appName);
            let children = 0;
            try {
                children = app.get_child_count();
            } catch (e) {
                children = 0;
            }
            for (let c = 0; c < children && c < 24; c++) {
                let frame = null;
                try {
                    frame = app.get_child_at_index(c);
                } catch (e) {
                    continue;
                }
                if (!frame)
                    continue;
                const name = accessibleText(frame);
                if (CALL_TEXT.test(name) || /whatsapp|snapchat|telegram|signal|discord|zoom|skype|messenger|teams/.test(name))
                    frames.push(frame);
            }
            if (!browser && /whatsapp|snapchat|telegram|signal|discord|zoom|skype/.test(appName))
                frames.push(app);
        }
        return frames;
    }

    _foreignApps(desktop) {
        const apps = [];
        let count = 0;
        try {
            count = desktop.get_child_count();
        } catch (e) {
            return apps;
        }
        for (let i = 0; i < count; i++) {
            let app = null;
            try {
                app = desktop.get_child_at_index(i);
            } catch (e) {
                continue;
            }
            if (!app)
                continue;
            const appName = accessibleText(app);
            if (/dyland|dyd-ui|gnome-shell|ibus/.test(appName))
                continue;
            apps.push(app);
        }
        return apps;
    }

    _collectCallButtons(node, wanted, kind, depth, budget, hits) {
        if (!node || depth > 30 || budget.seen > 8000 || hits.length > 8)
            return;
        budget.seen += 1;
        let role = -1;
        try {
            role = node.get_role();
        } catch (e) {
            role = -1;
        }
        if (role === Atspi.Role.PUSH_BUTTON || role === Atspi.Role.BUTTON || role === Atspi.Role.TOGGLE_BUTTON) {
            let showing = false;
            let enabled = false;
            try {
                const states = node.get_state_set();
                showing = states.contains(Atspi.StateType.SHOWING);
                enabled = states.contains(Atspi.StateType.SENSITIVE) || states.contains(Atspi.StateType.ENABLED);
            } catch (e) {
                showing = false;
            }
            const text = `${accessibleText(node)} ${attributeBlob(node)}`.toLowerCase();
            const inCall = CALL_TEXT.test(ancestorContext(node)) || CALL_APP_RE.test(ancestorContext(node)) || CALL_APP_RE.test(text);
            if (showing && enabled && wanted.test(text) && inCall) {
                let score = 1;
                if (kind === 'reject' && /\b(end(?:\s+call)?|leave(?:\s+call)?|hang)/.test(text))
                    score = 3;
                if (kind === 'accept' && /\b(accept|answer)\b/.test(text))
                    score = 3;
                hits.push({ node, score });
            }
        }
        let children = 0;
        try {
            children = node.get_child_count();
        } catch (e) {
            return;
        }
        if (skipCallBranch(role, children))
            return;
        const limit = Math.min(children, 160);
        for (let i = 0; i < limit; i++) {
            let child = null;
            try {
                child = node.get_child_at_index(i);
            } catch (e) {
                continue;
            }
            this._collectCallButtons(child, wanted, kind, depth + 1, budget, hits);
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
