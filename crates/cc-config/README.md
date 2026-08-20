# cc-config

Layered configuration for companiond: built-in defaults → TOML file →
environment variables → CLI overrides, merged per-field (a CLI flag
overrides only its own field, never a whole section), then validated as one
cross-field-consistent whole.

Pure and synchronous — no `tokio`, no sockets. The only I/O is one optional
config-file read; the merge itself is a pure function over three
already-parsed layers, so every precedence and validation rule is
unit-testable without touching the filesystem or the process environment.

## Scope

| In this crate | Not in this crate |
|---|---|
| Precedence merge (defaults → file → env → CLI) | Parsing CLI *flags themselves* — the binary parses argv into `Overrides`, this crate only merges it in |
| Cross-field validation (hysteresis ordering, non-zero IDs, …) | Using the resolved `Config` — every other crate/app just receives a `Config` value |
| Enum-string parsing (`"serial"` → `TransportKind::Serial`) | |

## Precedence model

```text
Config::default()  →  file (TOML)  →  env (CC_<SECTION>__<FIELD>)  →  CLI (Overrides)
    lowest                                                                 highest
```

Each layer is parsed independently into an all-`Option` mirror
(`PartialConfig`), then overlaid onto the previous layer (`hi.or(lo)` per
field — see `layer.rs`'s `overlay_fields!` macro), and the fully-overlaid
partial is finally applied onto `Config::default()`. Enum-valued fields
(`transport.kind`, `mission_log.compression`, `handshake.param_snapshot`)
travel as bare strings through every layer and are parsed in exactly **one**
place (`Config::apply`), so a bad value produces the identical
`ConfigError::Value` regardless of which layer it came from.

A **missing** config file is not an error — defaults, env, and CLI still
apply. A **present but malformed** file (bad TOML syntax, or an unknown
field — every partial struct is `#[serde(deny_unknown_fields)]`) is a hard
`ConfigError::File`.

## Public API

```rust
pub const DEFAULT_CONFIG_PATH: &str; // "/etc/companiond/config.toml"

pub enum ConfigError { File(String), Env(String), Value(String) }

impl Config {
    pub fn load(explicit_path: Option<&Path>, cli: PartialConfig) -> Result<Config, ConfigError>;
    pub fn merge(file: PartialConfig, env: PartialConfig, cli: PartialConfig) -> Result<Config, ConfigError>;
}
```

`load` is the impure entry point: resolves the file path (`--config` flag →
`$CC_CONFIG` → `DEFAULT_CONFIG_PATH`), reads the environment, and delegates
to `merge`, which is pure — the whole precedence engine in one testable
function, minus the two reads.

```rust
pub struct Overrides {
    pub vehicle_id: Option<u32>,
    pub mission_root: Option<String>,
    pub status_json: Option<bool>,
    pub transport_kind: Option<String>,
    pub udp_bind: Option<String>,
    pub remote: Option<String>,
    pub serial_path: Option<String>,
    pub baud: Option<u32>,
    pub sysid: Option<u8>,
    pub disk_floor_bytes: Option<u64>,
    pub param_snapshot: Option<String>,
}
impl Overrides {
    pub fn into_partial(self) -> PartialConfig;
}
```

`Overrides` is the flat, public surface a binary's argv parser fills in;
`into_partial` maps it onto `PartialConfig` (whose per-section structs are
crate-private — `Overrides` is the only supported way to construct a CLI
layer from outside the crate).

## Configuration model (`model::Config`)

Five sections, every field defaulted:

### `[general]`
| Field | Default | |
|---|---|---|
| `vehicle_id` | `1` | Must match the PX4 param (spec §7); `0` is rejected |
| `mission_root` | `/var/lib/companiond/missions` | Root under which mission directories are minted |
| `status_json` | `false` | Emit the machine-readable status line |

### `[transport]`
| Field | Default | |
|---|---|---|
| `kind` | `Udp` | `Udp` or `Serial` |
| `udp_bind` | `0.0.0.0:24040` | Local bind address |
| `remote` | `None` | Fixed peer; when absent, learned from the first validly-decoded frame |
| `serial_path` | `None` | Required if `kind = Serial` |
| `baud` | `921600` | |
| `sysid` | `1` | |

### `[mission_log]`
| Field | Default | |
|---|---|---|
| `flush_rows` | `5000` | Seal a Parquet part after this many rows (also the row-group size) |
| `flush_secs` | `10` | Seal a part after this many seconds even with no new rows |
| `seg_cap_bytes` | `2 GiB` | Roll to a new segment at this size |
| `seg_cap_secs` | `1800` (30 min) | …or this age |
| `raw_capture` | `true` | Append the length-prefixed raw-MAVLink ground-truth capture |
| `compression` | `Zstd` | `None`, `Snappy`, or `Zstd` |

### `[disk]` — the shedding ladder
| Field | Default | |
|---|---|---|
| `floor_bytes` | `5 GiB` | Refuse to *start* a mission below this |
| `raw_shed_low_bytes` / `raw_resume_bytes` | `2 GiB` / `4 GiB` | Stop/resume the raw capture |
| `bf_shed_low_bytes` / `bf_resume_bytes` | `1 GiB` / `1.5 GiB` | Stop/resume IMU + actuator rows |
| `crit_low_bytes` / `crit_resume_bytes` | `512 MiB` / `768 MiB` | Keep only never-shed classes + events + manifest |

### `[handshake]`
| Field | Default | |
|---|---|---|
| `context_hz` | `1.0` | `CC_MISSION_CONTEXT` publish rate |
| `stub_ack_on_heartbeat` | `true` | Accept on first FC heartbeat (Phase 6 flips this to wait for `CC_SAFETY_STATUS` leaving `UNKNOWN`) |
| `param_snapshot` | `Stub` | `Real`, `Stub`, or `Off` |
| `param_timeout_secs` | `20` | |

## Validation (`Config::validate`, run once after merge)

Every check has a matching unit test:

- `general.vehicle_id != 0`
- `transport.kind == Serial` requires `transport.serial_path` to be set
- `mission_log.flush_rows > 0` and `flush_secs > 0`
- `disk.floor_bytes >= mission_log.seg_cap_bytes` — otherwise a mission
  could never even open below the startup floor
- **Hysteresis**: each `*_resume_bytes` must sit strictly above its
  matching `*_shed_low_bytes`, for all three ladder stages (`raw`, `bf`,
  `crit`) — otherwise the shedding ladder would chatter at the threshold
  instead of hysteresing
- **Ladder ordering**: `crit_low_bytes < bf_shed_low_bytes <
  raw_shed_low_bytes` — so a falling free-space value crosses the stages in
  the documented sequence
- `handshake.context_hz > 0.0`

## Environment variable naming

`CC_<SECTION>__<FIELD>` (double underscore separates section from field),
uppercased — e.g. `CC_TRANSPORT__UDP_BIND`, `CC_DISK__FLOOR_BYTES`,
`CC_MISSION_LOG__COMPRESSION`. Values that fail to parse to their field's
type produce `ConfigError::Env`, distinct from a file-layer parse failure
(`ConfigError::File`) or a validation failure (`ConfigError::Value`).

## Testing

`lib.rs`'s test module (13 tests) covers: defaults resolve to the documented
values; each layer wins when it's the only one setting a field; CLI beats
env beats file on the *same* field; malformed TOML and unknown TOML fields
are hard errors; a nested env var parses into the right section/field;
an unparseable env value errors as `Env`; a bad enum string errors uniformly
as `Value` regardless of source layer; and each `validate()` rule
(zero vehicle ID, serial-without-path, floor-below-segment-cap,
non-hysteretic thresholds, unordered ladder) is exercised individually.

## Dependencies

`serde` (deserializing TOML into the partial structs), `toml`.
