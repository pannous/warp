# World events: the internet as a sensor of the world

Card world-events (user 2026-11-14): listen to, pull or subscribe to public event sources on the internet (Polymarket,
X/Twitter, Wikipedia, earthquakes …) with one warp spelling for all of them. Companions: notes/events_plan.md (the one
event loop; its Readable and Completion sources are what streaming needs), notes/web_framework.md "Async data".

## 1. Sources (probed 2026-11-14 from the Mac with curl; CORS = what a request with an Origin header got back)

Delivery: **push** = the server keeps a connection open (WebSocket, SSE = server-sent events, a text/event-stream of
`data:` lines over plain HTTP); **pull** = we ask again (REST polling, RSS). Native = the warp binary today, whose only
network word is fetch (net.get: curl, the whole answer, synchronous); page = the playground (a synchronous
XMLHttpRequest, so it needs CORS and can't hold a stream open).

| source | delivery | auth | cost | limits | native today | page today |
|---|---|---|---|---|---|---|
| Polymarket Gamma `gamma-api.polymarket.com/markets`, `/events` | pull, REST JSON | none | free | documented per-endpoint limits, hundreds of requests per 10 s | fetch ✓ | ✓ CORS * |
| Polymarket CLOB REST `clob.polymarket.com/price`, `/book`, `/prices-history` | pull | none to read (keys only to trade) | free | same family | fetch ✓ | ✓ CORS * |
| Polymarket CLOB WebSocket `wss://ws-subscriptions-clob.polymarket.com/ws/market` | push: send `{"assets_ids": [token ids], "type": "market"}`, get `book`, `price_change`, `last_trade_price` | none for the market channel (the user channel needs API keys) | free | ping every ~10 s keeps it open | ✗ no WebSocket word | ✗ no host word (browser WebSocket would do it) |
| X / Twitter API v2 | pull (search, timelines), push (filtered stream) | OAuth / bearer token, paid account | pay-per-use since 2026-02: $0.005 per post read, $0.015 per post written; subscriptions closed to new sign-ups; 3 M reads a month, then Enterprise ($42k+/month); no free read tier | per credit | fetch with a header: possible, costs money | ✗ (no CORS, key would leak) |
| Bluesky Jetstream `wss://jetstream2.us-east.bsky.network/subscribe?wantedCollections=app.bsky.feed.post` | push, WebSocket of JSON (the whole network's posts, likes, follows; filter by collection and DID) | none | free | none stated; whole firehose is ~hundreds of events/s | ✗ no WebSocket word (curl 8.7.1 here lacks ws://) | ✗ no host word (browser WebSocket would do it, no CORS for sockets) |
| Bluesky AppView `public.api.bsky.app/xrpc/…` | pull | none for profiles and author feeds; **searchPosts now answers 403** without a login | free | 3000 per 5 min per IP | fetch ✓ (not search) | no CORS header on search |
| Wikipedia EventStreams `stream.wikimedia.org/v2/stream/recentchange` (also page-create, revision-score …) | push, SSE; resumable with `Last-Event-ID` or `?since=` | none (send a User-Agent) | free | a few concurrent connections per IP | ✓ prototype: curl windows (below) | ✓ CORS *, EventSource would work: no host word yet |
| Hacker News Firebase `hacker-news.firebaseio.com/v0/…` | pull, and SSE with `Accept: text/event-stream` | none | free | none stated | fetch ✓ | ✓ CORS (echoes the origin) |
| GitHub `api.github.com/events`, `/repos/o/r/events` | pull; webhooks need a server we run | none (60 requests/h) or a token (5000/h) | free | `X-Poll-Interval` 60 s, ETag makes a 304 free; events lag 30 s to hours | fetch ✓ | ✓ CORS * |
| USGS earthquakes `earthquake.usgs.gov/earthquakes/feed/v1.0/summary/all_hour.geojson` | pull, GeoJSON updated each minute | none | free | none stated | fetch ✓ | ✓ CORS * |
| News RSS (BBC `feeds.bbci.co.uk/news/world/rss.xml`, Reuters via Google News RSS …) | pull, XML | none | free | polite polling (minutes) | fetch ✓, lib/xml parses | ✗ no CORS on most feeds: needs a proxy |
| GDELT `api.gdeltproject.org/api/v2/doc/doc` | pull, 15-minute world news index | none | free | ~1 request / 5 s | fetch ✓ | ✗ no CORS header |
| Crypto: Coinbase `api.coinbase.com/v2/prices/BTC-USD/spot`, Binance `api.binance.com/api/v3/ticker/price`, CoinGecko | pull REST; push WebSocket (`wss://ws-feed.exchange.coinbase.com` ticker, `wss://stream.binance.com:9443/ws/btcusdt@trade`) | none for public market data | free (CoinGecko keyless ~30/min) | per exchange | fetch ✓ | ✓ CORS * on all three REST |
| Weather: Open-Meteo `api.open-meteo.com/v1/forecast?current=…` | pull | none | free, non-commercial | 10 000 a day | fetch ✓ | ✓ CORS * |
| Reddit `reddit.com/r/x/new.json` | pull | OAuth for the API now; anonymous JSON is throttled or blocked | free tier small | 100/min with OAuth | flaky | ✗ no CORS |
| Mastodon streaming `/api/v1/streaming` | push, SSE or WebSocket | most instances want a token for public timelines now | free | per instance | — | — |

What it says:
- **Free, keyless and push**: Wikipedia (SSE), Hacker News (SSE), Bluesky Jetstream (WebSocket), Polymarket market
  channel (WebSocket), exchange tickers (WebSocket). Everything else is a poll.
- **X costs money per post read** and needs a token: not a default source. Bluesky Jetstream is the free stand-in for
  "watch Twitter"; X stays a library that takes a token from the user's settings, never from the repo.
- **Page**: most JSON APIs send `access-control-allow-origin: *`; RSS and GDELT don't, so the page needs a proxy
  (a later pannous.com endpoint) for them.
- **Native**: every pull source works today through fetch. No push source does: there is no word that keeps a
  connection open and hands over what arrives.

## 2. Plan: one spelling for every source

### A world source is a stream of events
Every source becomes one value, a **Feed**: a class with `next()`, which gives the next event (warp data: a map from
the JSON) or ø when the feed ends. The decided for-over-iterator rule (lib/channels.warp `next()`) makes it a loop
with no new syntax:

```warp
use wikipedia
for edit in wikipedia.changes(wiki: "enwiki") { print edit.title }

use polymarket
for price in polymarket.prices("will-x-happen") { print price }   // each change of the yes price
print polymarket.price("will-x-happen")                            // pull once: 0.42

use earthquakes
for quake in earthquakes.recent(magnitude: 4.5) { print quake.place }
```

Feeds are made by two general constructors in lib/feeds.warp, so a source module is a few lines naming its URL and
its event shape:
- `streamed(url)`: server-sent events, each `data:` line's JSON one event; reconnects with `Last-Event-ID`, so a
  dropped connection or a pull window loses nothing.
- `polled(fetching: function, every: duration, key: function)`: calls `fetching` each period and gives the items not
  seen before (by `key`, e.g. an id), or, for a single value, each change of it.
- later `socket(url, subscribe: message)`: a WebSocket, each message's JSON one event (Jetstream, Polymarket CLOB,
  exchange tickers).

Feeds compose like lists: `.first(n)`, `where`, `.each(f)`; a filter is a parameter or a comprehension
(`[e for e in feed if e.bot == no]` is endless, so the loop form with `break`/`first` is the one to show).

### `on`: the push spelling, once the event loop has Readable sources
`on wikipedia.changes { print event.title }` is a call `on(source: Feed, handler: block)` (plain warp, an overload by
type, Principles "Plain warp first"). It registers the feed as a Readable source with events_turn
(notes/events_plan.md section 3) and runs the block per event, while timers, frames and other feeds go on. Until the
loop exists a `for` is the honest spelling: it blocks, and says so.
`on polymarket.market("x").price change {…}` falls out of the same: `.price` is a polled Feed of changes. No rule is
needed for either.

### What exists, and what maps where
| need | there today | status |
|---|---|---|
| one-shot GET as warp data | lib/net.warp `fetch(url)` (curl natively, sync XHR in the page) | ✓ every pull source |
| request headers (tokens, User-Agent, Accept) on GET | `post(url, body, headers)` only | missing: `fetch(url, headers)` (lib + driver word: net.get takes a list today) |
| an iterator in `for` | `next()` (decision for-over-iterator) | ✓ Feed uses it |
| periodic pull | lib/timers.warp `on every`, lib/tasks.warp `sleep` | ✓ but timers only fire after main (events_plan "Today" 2) |
| one loop where feeds, timers, frames meet | notes/events_plan.md step events-loop, Readable source | planned: `on feed {…}` waits on it |
| a stream that stays open (SSE, WebSocket) | nothing: fetch reads to the end | **missing host words** (below) |
| JSON | lib/json.warp | ✓ |
| RSS / Atom | lib/xml.warp | ✓ parse; a `rss(url)` feed is a few lines |
| secrets (X token) | lib/os env / settings | a token never in the repo: read from `~/.config/warp/…` or an option |

### Missing pieces (cards)
1. **stream host words** (core/native/driver.warp + web/playground host JS + lib/net.warp declarations; core, so
   Interviewer first): `stream_open(url, headers): int`, `stream_next(id, wait_ms): text|ø` (one SSE data payload or
   one WebSocket message), `stream_send(id, text)`, `stream_close(id)`. Natively libcurl or a curl child with a pipe
   read non-blocking (one fd per stream: exactly the Readable source events_wait polls); in the page `EventSource`
   and `WebSocket` queuing into a SharedArrayBuffer like held keys. Native WebSocket needs libcurl ≥ 8.11 or a small
   C client in the native host library (lib/host/native_host, a library, not the driver).
2. **fetch with headers** (`fetch(url, headers)`), the twin of post's.
3. **`on feed {…}`** once events-loop lands: the overload above, registering the stream's fd.
4. **a CORS proxy** for RSS and GDELT in the page (pannous.com, server side).
5. Source modules beyond the prototype: bluesky (Jetstream, needs 1), hackernews (SSE), github, earthquakes, rss,
   crypto (ticker), weather, x (token, paid).

## 3. Prototype (lib only, native; landed with card world-events)
- lib/feeds.warp: the `Feed` base class (next(), take(n), keeps(e)) and three kinds a source extends: `Streamed`
  (server-sent events), `Polled` (items asked again each period, each given once by key) and `Changes` (a value asked
  again, given when it changes). Until the stream host words exist (card open-streams-host), `Streamed` reads the
  stream in curl windows of 3 s (lib/os `run`, the native warp only), resuming each from the last event id
  (`Last-Event-ID`), so nothing is lost between windows; the page has no `run` and refuses loudly.
- lib/wikipedia.warp `changes(wiki)`, lib/polymarket.warp `markets(n)`, `market(slug)`, `price(slug)`,
  `prices(slug, every)`, `outcome_prices(m)`, lib/earthquakes.warp `quakes(magnitude, every)`.
- samples/world/wikipedia_edits.warp, polymarket_odds.warp, strong_quakes.warp; test/world/test_world.pairs.warp
  (`needs: network`, about 7 s a line: each is a live request).

Spelling today: `for edit in changes("enwiki").take(5) {…}`. The endless `for edit in changes("enwiki") {…}` waits on
card for-over-iterator: the iterator rule fires only for a written constructor call, so a function giving a Feed falls
to the list rule, which walks the object's fields (card object-iterator-loop: it should refuse).

Found on the way (cards): spaced-unit (`30 s` in a module breaks the constant, function or field; `30s` works), a field
defaulting to a module constant is null, a sample named like a module `use`s itself. Gamma's market
list can answer from a cache a few seconds old; `price(slug)` asks by slug.
