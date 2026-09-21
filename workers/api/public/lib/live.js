// One socket, and a poll that still works when there is not one.
//
// WHY. A console open all day, a courier on shift and three customers watching
// their orders asked this platform about ten thousand questions a day, and
// nearly every answer was "nothing has changed". The hub knows the moment
// something happens; this is the wire it says so on.
//
// THE FALLBACK IS NOT AN AFTERTHOUGHT. A socket fails for reasons none of the
// three surfaces can do anything about -- a captive portal, a corporate proxy,
// a phone that slept -- so `live()` NEVER replaces the poll; it makes it slow.
// A caller keeps its interval and calls `tick()` from it; when the socket is
// up, `due()` says "not yet" until the socket has been quiet for longer than a
// socket should ever be, and when it is down the interval is what it always
// was. Nothing here can make a surface stop updating.
//
// THE TOKEN TRAVELS AS A SUBPROTOCOL. A browser cannot set a header on a
// WebSocket, and a token in the URL is a token in every log and every history;
// `new WebSocket(url, ['bearer', token])` is the standard way round it.

/// How long the socket may be silent before a caller polls anyway. The hub
/// sends nothing when nothing happens, so silence is the normal state; this is
/// the belt to the heartbeat's braces, and it is no longer the ONLY thing
/// standing between a surface and a socket that quietly died -- see PING_MS.
const QUIET_MS = 90_000;
/// Reconnect backoff: immediate, then slower, never faster than this.
const BACKOFF_MS = [1000, 2000, 5000, 15000, 30000];
/// HOW A DEAD SOCKET IS NOTICED AT ALL.
///
/// `state` only ever left 'live' on `onclose`, and a half-open socket -- a
/// phone that changed network, a laptop that slept, a middlebox that dropped
/// the flow -- never fires one. It therefore looked 'live' for ever, and
/// `due()` answered false, so the surface that trusted it fell back to ONE
/// read every 90 s instead of its own cadence. Measured on the owner console
/// with ten live orders: the first request after the network went away came
/// 90 s later, and the screen showed an "Open" chip over green Ready buttons
/// the whole time, because `isStale` needs fifteen minutes.
///
/// The hub has answered `{"t":"ping"}` with a pong since the day sockets
/// landed and no client had ever sent one. Now one does, and a socket that
/// does not answer two of them is closed -- which runs `onclose`, which puts
/// the surface back on its own poll and starts the reconnect backoff.
const PING_MS = 25_000;
const DEAD_MS = PING_MS * 2 + 5_000;

export function live({ token, onEvent, onState } = {}) {
  let ws = null;
  let attempt = 0;
  let lastHeard = 0;
  let closed = false;
  let heart = null;
  let state = 'connecting';

  const set = s => { if (s !== state) { state = s; onState?.(s); } };

  function open(){
    if (closed || !token) { set('off'); return; }
    let url;
    try {
      url = new URL('/api/live', location.origin);
      url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
    } catch { set('off'); return; }
    try {
      // The second subprotocol IS the token. The server echoes `bearer`.
      ws = new WebSocket(url, ['bearer', token]);
    } catch { set('off'); schedule(); return; }

    ws.onopen = () => { attempt = 0; lastHeard = Date.now(); set('live'); beat(); };
    ws.onmessage = e => {
      lastHeard = Date.now();
      let m; try { m = JSON.parse(e.data); } catch { return; }
      // `event` names an order; `moved` says only that the log changed -- a
      // placement, a rotation, anything that did not come through the append
      // path. Both mean "ask", which is all a caller does with either.
      if (m.t === 'event' || m.t === 'moved') onEvent?.(m);
    };
    ws.onerror = () => { /* onclose follows, and that is where we recover */ };
    ws.onclose = () => { ws = null; clearInterval(heart); heart = null; set('polling'); schedule(); };
  }

  /// Send a ping on a timer, and hang up on a socket that stopped answering.
  function beat(){
    clearInterval(heart);
    heart = setInterval(() => {
      if (closed || !ws) return;
      if (Date.now() - lastHeard > DEAD_MS) {
        // Not a graceful goodbye: this socket is already gone and only the
        // close event will tell the rest of the module so.
        try { ws.close(); } catch {}
        return;
      }
      try { ws.send(JSON.stringify({ t: 'ping' })); } catch { try { ws.close(); } catch {} }
    }, PING_MS);
  }

  function schedule(){
    if (closed) return;
    const wait = BACKOFF_MS[Math.min(attempt, BACKOFF_MS.length - 1)];
    attempt += 1;
    setTimeout(open, wait);
  }

  open();

  return {
    /// Is a poll due? A caller asks this from its own interval, so the
    /// decision lives in one place and a surface never stops updating because
    /// of something this module believes.
    due(){
      if (state !== 'live') return true;
      return Date.now() - lastHeard > QUIET_MS;
    },
    /// Note that the caller has just polled, whatever the reason.
    polled(){ lastHeard = Date.now(); },
    /// Send a courier's position. WHOSE it is comes from the socket's tag,
    /// which the server attached from the token: this frame carries only the
    /// coordinates, because a frame that named its own courier let one
    /// courier move another's pin.
    gps(latE6, lngE6){
      if (state !== 'live' || !ws) return false;
      try {
        ws.send(JSON.stringify({ t: 'gps', lat_e6: latE6, lng_e6: lngE6 }));
        return true;
      } catch { return false; }
    },
    get state(){ return state; },
    close(){ closed = true; clearInterval(heart); heart = null; try { ws?.close(); } catch {} ws = null; set('off'); },
  };
}
