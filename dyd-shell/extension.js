/* Dyland dynamic island: a phone-style island rendered inside the GNOME
 * Shell UI. It sits at the very top of the screen (overlapping the panel,
 * exactly where the clock lives) and is always visible regardless of which
 * window has focus.
 *
 * Data comes from the dyd-daemon over a newline-delimited JSON protocol on
 * the unix socket /tmp/dyd-daemon.sock (same protocol the Tauri UI used).
 */

const { Clutter, GLib, Gio, Pango, St } = imports.gi;

const Main = imports.ui.main;

const SOCKET_PATH = '/tmp/dyd-daemon.sock';
const RECONNECT_MS = 2000;
const IDLE_MS = 4000;
const NOTIFICATION_DISMISS_MS = 5000;
const ACTIVITY_DISMISS_MS = 6000;
const TOP_MARGIN = 2;
const ANCHOR_HEIGHT = 320;

const SIZES = {
    dormant: { width: 126, height: 40 },
    active: { width: 180, height: 40 },
    notification: { width: 200, height: 40 },
    media: { width: 230, height: 40 },
    expanded: { width: 380, height: 200 },
};

var DaemonClient = class DaemonClient {
    constructor(onEvent, onStatus) {
        this._onEvent = onEvent;
        this._onStatus = onStatus;
        this._connection = null;
        this._stream = null;
        this._reconnectId = 0;
        this._connected = false;
    }

    start() {
        this._connect();
    }

    stop() {
        if (this._reconnectId) {
            GLib.source_remove(this._reconnectId);
            this._reconnectId = 0;
        }
        this._close();
    }

    isConnected() {
        return this._connected;
    }

    _connect() {
        const address = new Gio.UnixSocketAddress({ path: SOCKET_PATH });
        const client = new Gio.SocketClient();
        client.connect_async(address, null, (c, res) => {
            let connection;
            try {
                connection = c.connect_finish(res);
            } catch (e) {
                this._connected = false;
                this._onStatus('disconnected');
                this._scheduleReconnect();
                return;
            }
            this._connection = connection;
            this._stream = new Gio.DataInputStream({
                base_stream: connection.get_input_stream(),
                close_base_stream: true,
            });
            this._connected = true;
            this._onStatus('connected');
            this._readNextLine();
        });
    }

    _scheduleReconnect() {
        if (this._reconnectId)
            return;
        this._reconnectId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, RECONNECT_MS, () => {
            this._reconnectId = 0;
            this._connect();
            return GLib.SOURCE_REMOVE;
        });
    }

    _readNextLine() {
        if (!this._stream)
            return;
        this._stream.read_line_async(GLib.PRIORITY_DEFAULT, null, (stream, res) => {
            let line;
            try {
                [line] = stream.read_line_finish(res);
            } catch (e) {
                this._handleDisconnect();
                return;
            }
            if (line === null) {
                this._handleDisconnect();
                return;
            }
            try {
                const envelope = JSON.parse(line);
                this._onEvent(envelope);
            } catch (e) {
                log(`dyland: failed to parse daemon frame: ${e}`);
            }
            this._readNextLine();
        });
    }

    _handleDisconnect() {
        this._close();
        this._onStatus('disconnected');
        this._scheduleReconnect();
    }

    _close() {
        if (this._stream) {
            try {
                this._stream.close(null);
            } catch (e) {
            }
            this._stream = null;
        }
        this._connection = null;
        this._connected = false;
    }

    sendCommand(command) {
        if (!this._connected || !this._connection)
            return;
        const envelope = {
            id: GLib.uuid_string_random(),
            timestamp: Date.now() / 1000,
            kind: 'command',
            ...command,
        };
        const bytes = new TextEncoder().encode(JSON.stringify(envelope) + '\n');
        try {
            this._connection.get_output_stream().write_all(bytes, null);
        } catch (e) {
            log(`dyland: send failed: ${e}`);
        }
    }
};

var Island = class Island {
    constructor(client) {
        this._client = client;
        this._state = 'dormant';
        this._fullscreen = false;
        this._locked = false;
        this._connected = false;
        this._notification = null;
        this._media = null;
        this._activity = null;
        this._status = {
            battery: null,
            wifi: null,
            bluetooth: null,
            volume: null,
            brightness: null,
        };
        this._idleId = 0;
        this._notificationTimer = 0;
        this._activityTimers = new Map();

        this._anchor = new St.Widget({
            layout_manager: new Clutter.BinLayout(),
            reactive: false,
            style_class: 'dyd-anchor',
        });

        this._pill = new St.BoxLayout({
            style_class: 'dyd-pill',
            reactive: true,
            track_hover: true,
            can_focus: true,
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.START,
            clip_to_allocation: true,
        });
        this._pill.set_margin_top(TOP_MARGIN);

        this._content = new St.BoxLayout({
            vertical: false,
            style_class: 'dyd-content',
            reactive: false,
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
            x_expand: true,
            y_expand: true,
        });

        this._pill.add_actor(this._content);
        this._anchor.add_actor(this._pill);
        Main.uiGroup.add_actor(this._anchor);

        this._pill.connect('button-press-event', this._onPillPress.bind(this));
        global.stage.connect('button-press-event', this._onStagePress.bind(this));
        Main.layoutManager.connect('monitors-changed', this._reposition.bind(this));
        Main.sessionMode.connect('updated', this._onSessionModeChanged.bind(this));

        this._reposition();
        this._applySize();
        this._render();
    }

    destroy() {
        if (this._idleId) {
            GLib.source_remove(this._idleId);
            this._idleId = 0;
        }
        if (this._notificationTimer) {
            GLib.source_remove(this._notificationTimer);
            this._notificationTimer = 0;
        }
        for (const id of this._activityTimers.values())
            GLib.source_remove(id);
        this._activityTimers.clear();
        this._anchor.destroy();
    }

    /* --- geometry ---------------------------------------------------- */

    _reposition() {
        const monitor = Main.layoutManager.primaryMonitor;
        this._anchor.set_position(monitor.x, monitor.y);
        this._anchor.set_size(monitor.width, ANCHOR_HEIGHT);
    }

    _applySize() {
        if (this._state === 'hidden') {
            this._pill.ease_property('width', 0, {
                duration: 120,
                mode: Clutter.AnimationMode.EASE_OUT_QUAD,
            });
            this._pill.ease_property('height', 0, {
                duration: 120,
                mode: Clutter.AnimationMode.EASE_OUT_QUAD,
            });
            this._pill.ease_property('opacity', 0, {
                duration: 120,
                mode: Clutter.AnimationMode.EASE_OUT_QUAD,
            });
            return;
        }
        const size = this._currentSize();
        const duration = this._state === 'expanded' ? 320 : 240;
        this._pill.ease_property('width', size.width, {
            duration,
            mode: Clutter.AnimationMode.EASE_OUT_CUBIC,
        });
        this._pill.ease_property('height', size.height, {
            duration,
            mode: Clutter.AnimationMode.EASE_OUT_CUBIC,
        });
        this._pill.ease_property('opacity', 1, {
            duration: 120,
            mode: Clutter.AnimationMode.EASE_OUT_QUAD,
        });
    }

    _currentSize() {
        if (this._notification && (this._state === 'dormant' || this._state === 'active'))
            return SIZES.notification;
        if (this._media && this._media.playbackStatus === 'playing' &&
            (this._state === 'dormant' || this._state === 'active'))
            return SIZES.media;
        if (this._activity && (this._state === 'dormant' || this._state === 'active'))
            return SIZES.notification;
        if (this._state === 'expanded')
            return SIZES.expanded;
        if (this._state === 'active')
            return SIZES.active;
        return SIZES.dormant;
    }

    /* --- state machine ------------------------------------------------ */

    _transitionTo(target) {
        const rules = {
            dormant: ['active', 'hidden'],
            active: ['dormant', 'expanded', 'hidden'],
            expanded: ['active', 'hidden'],
            hidden: ['dormant', 'active', 'expanded'],
        };
        if (!(rules[this._state] || []).includes(target))
            return;
        if (this._state === target)
            return;
        this._state = target;
        if (target === 'active')
            this._resetIdleTimer();
        this._applySize();
        this._render();
    }

    _resetIdleTimer() {
        if (this._idleId) {
            GLib.source_remove(this._idleId);
            this._idleId = 0;
        }
        this._idleId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, IDLE_MS, () => {
            this._idleId = 0;
            if (this._state === 'active' && !this._fullscreen && !this._locked)
                this._transitionTo('dormant');
            return GLib.SOURCE_REMOVE;
        });
    }

    _handleFullscreen(fullscreen) {
        if (this._fullscreen === fullscreen)
            return;
        this._fullscreen = fullscreen;
        if (fullscreen && this._state !== 'hidden')
            this._transitionTo('hidden');
        else if (!fullscreen && this._state === 'hidden')
            this._transitionTo('dormant');
    }

    _onSessionModeChanged() {
        const mode = Main.sessionMode.currentMode;
        const locked = mode === 'lock' || mode === 'unlock-dialog' || mode === 'greeter';
        if (locked === this._locked)
            return;
        this._locked = locked;
        if (locked)
            this._pill.set_opacity(0);
        else
            this._pill.set_opacity(1);
    }

    /* --- daemon events ------------------------------------------------ */

    _onDaemonStatus(status) {
        const connected = status === 'connected';
        if (connected === this._connected)
            return;
        this._connected = connected;
        if (connected && this._state === 'hidden' && !this._fullscreen)
            this._transitionTo('dormant');
        this._render();
    }

    _onDaemonEvent(env) {
        const type = env.type;
        const p = env.payload;
        switch (type) {
            case 'FullscreenChanged':
                if (p && typeof p.is_fullscreen === 'boolean')
                    this._handleFullscreen(p.is_fullscreen);
                break;
            case 'NotificationReceived':
                if (p && p.app_name) {
                    this._notification = p;
                    if (this._notificationTimer) {
                        GLib.source_remove(this._notificationTimer);
                        this._notificationTimer = 0;
                    }
                    this._notificationTimer = GLib.timeout_add(
                        GLib.PRIORITY_DEFAULT,
                        NOTIFICATION_DISMISS_MS,
                        () => {
                            this._notificationTimer = 0;
                            this._notification = null;
                            if (this._state === 'active' || this._state === 'expanded')
                                this._transitionTo('dormant');
                            this._render();
                            return GLib.SOURCE_REMOVE;
                        }
                    );
                    if (!this._fullscreen && !this._locked &&
                        (this._state === 'dormant' || this._state === 'hidden'))
                        this._transitionTo('active');
                    this._render();
                }
                break;
            case 'MediaChanged':
                if (!p || p.playback_status === 'stopped' || (!p.title && !p.artist) || !p.player)
                    this._media = null;
                else
                    this._media = p;
                this._render();
                break;
            case 'ActivityQueued':
                if (p && p.activity_id) {
                    this._activity = p;
                    this._render();
                }
                break;
            case 'ActivityDequeued':
                if (p && this._activity && p.activity_id === this._activity.activity_id) {
                    this._activity = null;
                    this._render();
                }
                break;
            case 'QueueEmptied':
                this._activity = null;
                this._render();
                break;
            case 'PluginActivity':
                if (p && p.activity_id) {
                    this._activity = p;
                    this._render();
                    if (!this._fullscreen && !this._locked &&
                        (this._state === 'dormant' || this._state === 'hidden'))
                        this._transitionTo('active');
                    const existing = this._activityTimers.get(p.activity_id);
                    if (existing)
                        GLib.source_remove(existing);
                    const id = GLib.timeout_add(
                        GLib.PRIORITY_DEFAULT,
                        ACTIVITY_DISMISS_MS,
                        () => {
                            this._activityTimers.delete(p.activity_id);
                            if (this._activity && p.activity_id === this._activity.activity_id) {
                                this._activity = null;
                                if (this._state === 'active' || this._state === 'expanded')
                                    this._transitionTo('dormant');
                                this._render();
                            }
                            return GLib.SOURCE_REMOVE;
                        }
                    );
                    this._activityTimers.set(p.activity_id, id);
                }
                break;
            case 'BatteryChanged':
                if (p && typeof p.is_present === 'boolean')
                    this._status.battery = p;
                this._render();
                break;
            case 'WifiChanged':
                if (p && typeof p.state === 'string')
                    this._status.wifi = p;
                this._render();
                break;
            case 'BluetoothChanged':
                if (p && typeof p.powered === 'boolean')
                    this._status.bluetooth = p;
                this._render();
                break;
            case 'VolumeChanged':
                if (p && typeof p.volume === 'number')
                    this._status.volume = p;
                this._render();
                break;
            case 'BrightnessChanged':
                if (p && typeof p.has_backlight === 'boolean')
                    this._status.brightness = p;
                this._render();
                break;
            default:
                break;
        }
    }

    /* --- interaction --------------------------------------------------- */

    _onPillPress(actor, event) {
        if (event.get_button() !== 1)
            return Clutter.EVENT_PROPAGATE;
        if (this._locked)
            return Clutter.EVENT_PROPAGATE;
        if (this._state === 'expanded') {
            this._transitionTo('active');
        } else if (this._state === 'active' ||
            (this._state === 'dormant' &&
                (this._notification || this._activity ||
                    (this._media && this._media.playbackStatus === 'playing')))) {
            this._transitionTo('expanded');
        }
        return Clutter.EVENT_STOP;
    }

    _onStagePress(actor, event) {
        if (this._state !== 'expanded')
            return Clutter.EVENT_PROPAGATE;
        const source = event.get_source();
        if (source && this._pill.contains(source))
            return Clutter.EVENT_PROPAGATE;
        this._transitionTo('active');
        return Clutter.EVENT_PROPAGATE;
    }

    /* --- content -------------------------------------------------------- */

    _render() {
        let content;
        if (!this._connected) {
            content = this._buildOffline();
            this._applySize();
        } else if (this._state === 'expanded') {
            content = this._buildExpanded();
            this._applySize();
        } else {
            content = this._buildCompact();
            this._applySize();
        }
        this._content.destroy_all_children();
        this._content.add_actor(content);
    }

    _icon(name) {
        return new St.Icon({ icon_name: name, style_class: 'dyd-icon' });
    }

    _label(text, styleClass) {
        return new St.Label({
            text,
            style_class: styleClass || 'dyd-label',
            x_align: Clutter.ActorAlign.START,
            y_align: Clutter.ActorAlign.CENTER,
        });
    }

    _ellipsized(text) {
        const label = this._label(text);
        label.clutter_text.ellipsize = Pango.EllipsizeMode.END;
        return label;
    }

    _row(children) {
        const row = new St.BoxLayout({
            vertical: false,
            style_class: 'dyd-row',
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
        });
        for (const child of children)
            row.add_actor(child);
        return row;
    }

    _buildOffline() {
        return this._row([this._icon('dialog-warning-symbolic'), this._label('Daemon offline')]);
    }

    _buildCompact() {
        if (this._notification)
            return this._buildNotificationLine();
        if (this._activity)
            return this._buildActivityLine();
        if (this._media && this._media.playbackStatus === 'playing')
            return this._buildMediaLine();
        return this._buildStatusLine();
    }

    _buildNotificationLine() {
        const row = this._row([
            this._appIcon(this._notification),
            this._ellipsized(this._notification.app_name || 'Notification'),
            this._ellipsized(this._notification.summary || ''),
        ]);
        return row;
    }

    _buildActivityLine() {
        const row = this._row([
            this._icon('process-working-symbolic'),
            this._ellipsized(this._activity.source || 'Activity'),
            this._ellipsized(this._activityText(this._activity)),
        ]);
        return row;
    }

    _buildMediaLine() {
        const row = this._row([
            this._icon(this._media.playbackStatus === 'paused' ?
                'media-playback-pause-symbolic' : 'media-playback-start-symbolic'),
            this._ellipsized(this._media.title || ''),
            this._ellipsized(this._media.artist || ''),
        ]);
        return row;
    }

    _buildStatusLine() {
        const children = [];
        const battery = this._status.battery;
        if (battery && battery.is_present) {
            children.push(this._icon(this._batteryIcon(battery.percentage)));
            children.push(this._label(`${Math.round(battery.percentage)}%`));
        }
        const wifi = this._status.wifi;
        if (wifi && wifi.state === 'connected')
            children.push(this._icon('network-wireless-signal-excellent-symbolic'));
        else if (wifi)
            children.push(this._icon('network-wireless-offline-symbolic'));
        const bluetooth = this._status.bluetooth;
        if (bluetooth && bluetooth.powered && bluetooth.present)
            children.push(this._icon('bluetooth-active-symbolic'));
        const volume = this._status.volume;
        if (volume)
            children.push(this._icon(volume.muted ? 'audio-volume-muted-symbolic' : 'audio-volume-high-symbolic'));
        if (children.length === 0)
            children.push(this._label('dyland'));
        return this._row(children);
    }

    _buildExpanded() {
        if (this._notification)
            return this._buildNotificationExpanded();
        if (this._activity)
            return this._buildActivityExpanded();
        if (this._media && this._media.playbackStatus !== 'stopped')
            return this._buildMediaExpanded();
        return this._buildStatusExpanded();
    }

    _buildNotificationExpanded() {
        const n = this._notification;
        const column = new St.BoxLayout({
            vertical: true,
            style_class: 'dyd-column',
            x_align: Clutter.ActorAlign.FILL,
            y_align: Clutter.ActorAlign.CENTER,
            x_expand: true,
            y_expand: true,
        });
        column.add_actor(this._label(n.app_name || 'Notification', 'dyd-label-title'));
        column.add_actor(this._label(n.summary || '', 'dyd-label'));
        if (n.body)
            column.add_actor(this._label(n.body, 'dyd-label-muted'));
        const actions = new St.BoxLayout({
            vertical: false,
            style_class: 'dyd-row-wide',
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
        });
        for (const action of n.actions || []) {
            const button = new St.Button({
                label: action.label,
                style_class: 'dyd-btn',
                can_focus: true,
            });
            button.connect('clicked', () => {
                this._client.sendCommand({
                    type: 'NotificationAction',
                    payload: { id: n.id, action_key: action.key, reply_text: null },
                });
            });
            actions.add_actor(button);
        }
        const close = new St.Button({ label: 'Close', style_class: 'dyd-btn', can_focus: true });
        close.connect('clicked', () => {
            this._client.sendCommand({ type: 'NotificationClose', payload: { id: n.id } });
            this._notification = null;
            this._transitionTo('active');
            this._render();
        });
        actions.add_actor(close);
        column.add_actor(actions);
        return column;
    }

    _buildActivityExpanded() {
        const a = this._activity;
        const column = new St.BoxLayout({
            vertical: true,
            style_class: 'dyd-column',
            x_align: Clutter.ActorAlign.FILL,
            y_align: Clutter.ActorAlign.CENTER,
            x_expand: true,
            y_expand: true,
        });
        column.add_actor(this._label(a.source || 'Activity', 'dyd-label-title'));
        column.add_actor(this._label(this._activityText(a), 'dyd-label'));
        const dismiss = new St.Button({ label: 'Dismiss', style_class: 'dyd-btn', can_focus: true });
        dismiss.connect('clicked', () => {
            this._client.sendCommand({ type: 'DismissActivity', payload: { activity_id: a.activity_id } });
            this._activity = null;
            this._transitionTo('active');
            this._render();
        });
        column.add_actor(dismiss);
        return column;
    }

    _buildMediaExpanded() {
        const m = this._media;
        const column = new St.BoxLayout({
            vertical: true,
            style_class: 'dyd-column',
            x_align: Clutter.ActorAlign.FILL,
            y_align: Clutter.ActorAlign.CENTER,
            x_expand: true,
            y_expand: true,
        });
        column.add_actor(this._label(m.title || '', 'dyd-label-title'));
        column.add_actor(this._label(m.artist || '', 'dyd-label-muted'));
        const progress = new St.Widget({ style_class: 'dyd-progress' });
        progress.set_width(Math.round(240 * (m.length > 0 ? Math.min(m.position / m.length, 1) : 0)));
        column.add_actor(progress);
        const controls = new St.BoxLayout({
            vertical: false,
            style_class: 'dyd-row-wide',
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
        });
        const prev = new St.Button({ label: '⏮', style_class: 'dyd-btn', can_focus: true });
        prev.connect('clicked', () => {
            this._client.sendCommand({ type: 'MediaPrevious', payload: { player: m.player } });
        });
        const playPause = new St.Button({
            label: m.playbackStatus === 'paused' ? '▶' : '⏸',
            style_class: 'dyd-btn',
            can_focus: true,
        });
        playPause.connect('clicked', () => {
            this._client.sendCommand({ type: 'MediaPlayPause', payload: { player: m.player } });
        });
        const next = new St.Button({ label: '⏭', style_class: 'dyd-btn', can_focus: true });
        next.connect('clicked', () => {
            this._client.sendCommand({ type: 'MediaNext', payload: { player: m.player } });
        });
        controls.add_actor(prev);
        controls.add_actor(playPause);
        controls.add_actor(next);
        column.add_actor(controls);
        return column;
    }

    _buildStatusExpanded() {
        const rows = [];
        const battery = this._status.battery;
        if (battery && battery.is_present) {
            rows.push(this._label(
                `Battery  ${Math.round(battery.percentage)}%  ${battery.state === 'charging' ? '(charging)' : ''}`));
        }
        const wifi = this._status.wifi;
        if (wifi && wifi.state === 'connected')
            rows.push(this._label(`Wi-Fi  ${wifi.ssid || 'connected'}`));
        else if (wifi)
            rows.push(this._label('Wi-Fi  offline'));
        const bluetooth = this._status.bluetooth;
        if (bluetooth && bluetooth.present && bluetooth.powered) {
            const connected = (bluetooth.devices || []).filter(d => d.connected);
            rows.push(this._label(`Bluetooth  ${connected.length > 0 ?
                connected.map(d => d.name).join(', ') : 'on'}`));
        }
        const volume = this._status.volume;
        if (volume)
            rows.push(this._label(`Volume  ${Math.round(volume.volume * 100)}%${volume.muted ? ' (muted)' : ''}`));
        const brightness = this._status.brightness;
        if (brightness && brightness.has_backlight)
            rows.push(this._label(`Brightness  ${brightness.brightness}%`));
        if (rows.length === 0)
            rows.push(this._label('No status data'));
        const column = new St.BoxLayout({
            vertical: true,
            style_class: 'dyd-column',
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
        });
        for (const label of rows)
            column.add_actor(label);
        return column;
    }

    /* --- helpers -------------------------------------------------------- */

    _appIcon(notification) {
        const path = notification.app_icon;
        if (path) {
            try {
                const file = Gio.File.new_for_path(path);
                if (file.query_exists(null))
                    return new St.Icon({
                        gicon: new Gio.FileIcon({ file }),
                        style_class: 'dyd-icon',
                    });
            } catch (e) {
            }
        }
        return this._icon('application-x-executable-symbolic');
    }

    _activityText(activity) {
        const content = activity.content || {};
        return content.title || content.text || content.message || JSON.stringify(content);
    }

    _batteryIcon(percentage) {
        const level = Math.max(0, Math.min(100, Math.round(percentage)));
        const thresholds = [10, 20, 30, 40, 50, 60, 70, 80, 90];
        for (const t of thresholds) {
            if (level <= t)
                return `battery-level-${t}-symbolic`;
        }
        return 'battery-level-100-symbolic';
    }
};

let _island = null;
let _client = null;

function init() {
}

function enable() {
    _client = new DaemonClient(
        env => _island && _island._onDaemonEvent(env),
        status => _island && _island._onDaemonStatus(status)
    );
    _island = new Island(_client);
    _client.start();
}

function disable() {
    if (_client) {
        _client.stop();
        _client = null;
    }
    if (_island) {
        _island.destroy();
        _island = null;
    }
}
