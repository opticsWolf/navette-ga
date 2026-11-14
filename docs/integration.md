# Integrating `navette-ga` with the Navette repository

`navette-ga` is **one Cargo package** (library `navette_ga` + binary
`navette-ga`). The subsystems are modules under `src/`: `decoder`, `bridge`,
`engines`, `qd`, `study`, `bench` (feature-gated), and the binary in
`src/main.rs`.

There is no `[workspace]` table and no generated manifest, so the package can
be moved or linked as one unit.

---

## 1. Add directly to the Navette repository (recommended)

Copy the package into `Navette/rust/navette-ga/`:

```powershell
# PowerShell, from the navette_ga repository root
New-Item -ItemType Directory -Force ..\Navette\rust\navette-ga | Out-Null
Copy-Item Cargo.toml, README.md, CREDIT.md, COPYING, COPYING.LESSER ..\Navette\rust\navette-ga\
Copy-Item -Recurse -Path src, staging -Destination ..\Navette\rust\navette-ga\
```

```sh
# POSIX
mkdir -p ../Navette/rust/navette-ga
cp Cargo.toml README.md CREDIT.md COPYING COPYING.LESSER ../Navette/rust/navette-ga/
cp -r src staging ../Navette/rust/navette-ga/
```

Change the navette dependency path in the copied `Cargo.toml` from the
sibling layout to the in-repo engine:

```toml
# Navette/rust/navette-ga/Cargo.toml
navette = { version = "0.10.0", path = "../navette", features = ["opt-basin"] }
```

Add the package to `Navette/Cargo.toml`:

```toml
members = [
  "rust/navette",
  "rust/navette-py",
  "rust/navette-ga",
]
```

Then:

```sh
cd ../Navette
cargo check --workspace
cargo test --workspace
cargo test -p navette-ga --features bench   # if the toolchain supports bench
```

No version/license/dependency table merge is needed: the package declares its
own `version`, `edition`, `rust-version`, `license`, dependencies, and binary
target. The only edit is the one `navette` path line above.

If you prefer a submodule or subtree instead of a copy, the same steps apply:
check it out at `Navette/rust/navette-ga` and change that one path.

---

## 2. Keep the repository separate and link it

### 2a. Path dependency (both checkouts side by side)

Navette can depend on the package directly:

```toml
# Navette/Cargo.toml or a member manifest
navette-ga = { path = "../navette_ga" }
```

The package's own `navette` path (`../Navette/rust/navette`) resolves to the
same canonical engine directory as Navette's `rust/navette`. If Cargo ever
reports two distinct navette packages in this mode, make the GA dependency
version-only (`navette = "0.10.0"`) and add the local patch to Navette's root
manifest:

```toml
[patch.crates-io]
navette = { path = "rust/navette" }
```

### 2b. Git dependency

A Cargo git dependency clones only this repository, so the sibling path
`../Navette/rust/navette` is not present in the checkout. For a git
dependency, use a version-only navette dependency in this package:

```toml
navette = { version = "0.10.0", features = ["opt-basin"] }
```

and patch it to the local engine in Navette's root manifest (as in 2a). Then:

```toml
navette-ga = { git = "https://github.com/opticsWolf/navette-ga" }
```

A normal published release does not have this problem: `cargo publish` strips
the `path` from a `{ version, path }` dependency and keeps the version, so the
crates.io artifact is self-contained.

### 2c. Published crate

Once published, the cleanest link is simply:

```toml
navette-ga = "0.1"
```

The `bench` feature is opt-in:

```toml
navette-ga = { version = "0.1", features = ["bench"] }
```

---

## 3. What not to move

- `research/` (PDFs, `kb.db`, probe output) and `vendor/` (pinned upstream
  mirrors) are local-only and git-ignored.
- `docs/` is the research/strategy record; it does not have to move into the
  Navette repository, but can be kept as a subdirectory if desired.
- `staging/STAGED-COPIES.md` is the E09 v2 §1.1 copy gate; it should travel
  with the code because `CREDIT.md` refers to it.
