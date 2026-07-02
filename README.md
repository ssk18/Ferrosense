# FerroSense

**A fast, robust Rust decoder for a streaming binary protocol — callable from Kotlin/Java on Android.**

Bytes arrive over a BLE/NFC link in arbitrary chunks: half a packet, one packet, or several
glued together. FerroSense frames them, validates each packet's checksum, and parses the payload
into typed readings — tracking sequence numbers and partial packets across the stream, without
panicking on malformed input.

```
raw bytes  ──▶  [ frame ]  ──▶  [ validate CRC ]  ──▶  [ parse ]  ──▶  typed Reading
 (chunks)        boundaries      reject corruption     payload → value
```

## Protocol

All multi-byte fields are **little-endian**.

```
┌────────┬────────┬──────────┬──────────────┬────────┬────────┐
│ 0xAA   │ length │ msg_type │ payload      │ seq    │ crc16  │
│ 1 byte │ 1 byte │ 1 byte   │ length bytes │ 2 bytes│ 2 bytes│
└────────┴────────┴──────────┴──────────────┴────────┴────────┘
  sync     N        type       data           counter  checksum
```

| Field      | Size      | Description                                            |
|------------|-----------|--------------------------------------------------------|
| `sync`     | 1 byte    | Start-of-frame marker, always `0xAA`                   |
| `length`   | 1 byte    | Payload length `N` (0–255)                             |
| `msg_type` | 1 byte    | Message type (see below)                               |
| `payload`  | `N` bytes | Type-specific data                                     |
| `seq`      | 2 bytes   | Monotonic sequence counter (`u16`) for gap detection   |
| `crc16`    | 2 bytes   | CRC-16/CCITT over `length … seq` (excludes sync + crc) |

A complete frame is `N + 7` bytes.

| `msg_type` | Reading       | Payload                 |
|------------|---------------|-------------------------|
| `0x01`     | `Temperature` | `i16` — centi-degrees C |
| `0x02`     | `Battery`     | `u8` — percent          |
| `0x03`     | `Heartbeat`   | *(empty)*               |

All three are values a phone can genuinely produce, so the protocol is end-to-end testable on real hardware (see below).

## Objectives

- Accept raw byte buffers exactly as delivered by BLE/NFC on Android.
- Frame a continuous stream into packets, resynchronising after garbage.
- Validate each packet with CRC-16 and drop corrupted frames.
- Parse payloads into strongly-typed readings.
- Track sequence numbers and buffer partial packets across the stream.
- Expose a clean API callable from Kotlin/Java on Android.
- Stay panic-free on malformed input; fully tested, fuzzed, and benchmarked.

## Architecture

A Cargo workspace whose root is a **virtual manifest** — it declares the member crates but is
**not itself a crate** (the analogue of `settings.gradle.kts`). One-way dependency: `ffi` → `core`.

```
ferrosense/
├── Cargo.toml          # virtual manifest: [workspace] members, no [package]
└── crates/
    ├── core/           # pure Rust: framing, CRC, parsing, decoder state — no platform deps
    └── ffi/            # thin JVM bridge (cdylib → .so); the only crate that targets Android
```

## Building

```bash
cargo check            # fast type + borrow check, no binary
cargo build --release
cargo test
```

## Testing on a real device

The decoder is **transport-agnostic** — it consumes byte buffers wherever they come from.
That gives three levels of on-device testing, cheapest first:

1. **Direct injection (no radio).** Feed synthetic frames — valid *and* deliberately corrupted —
   straight into the API from an instrumented test or a debug screen. One phone, fully
   deterministic, exercises framing, CRC, and parsing without any wireless setup.

2. **BLE — phone as the peripheral.** An Android phone can play the emitter: advertise a service
   with a notify characteristic (`BluetoothLeAdvertiser` + `BluetoothGattServer`, API 21+) and push
   FerroSense frames; a second phone in central mode receives the raw bytes and decodes them. The
   emitter can source **real** values from the phone itself (`BatteryManager` → battery percent and
   temperature) plus a periodic heartbeat. Peripheral/advertising support is device-dependent —
   check `BluetoothAdapter.isMultipleAdvertisementSupported()`; an nRF/ESP32 board works as the
   emitter too.

3. **NFC — phone as the tag.** Host Card Emulation (`HostApduService`, API 19+) lets a phone emulate
   an NFC target that a reader phone taps to pull a batch of bytes. NFC is tap/burst rather than
   streaming, so it exercises the single-buffer path instead of partial-packet reassembly.

All three hit the same entry point; framing, CRC, and parsing are identical regardless of transport.
