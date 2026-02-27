# CI/CD Pipeline

Two separate workflows manage builds and releases.

## CI Workflow (`ci.yml`)

**Triggers:**
- Push to `main` branch
- Pull requests to `main`

**Jobs:**
1. **test** - Runs `cargo test --lib` on Ubuntu
2. **build** - Builds release binaries for:
   - macOS Apple Silicon (`aarch64-apple-darwin`)
   - macOS Intel (`x86_64-apple-darwin`)
   - Linux x86_64 (`x86_64-unknown-linux-gnu`)

## Release Workflow (`release.yml`)

**Triggers:**
- Tag push matching `v*` (stable releases)
- Tag push matching `v*-beta*` (beta/prerelease)

**Process:**
1. Runs tests
2. Builds and strips binaries
3. Creates tarball archives
4. Uploads artifacts
5. Creates GitHub Release with auto-generated notes
6. Marks as prerelease if tag contains "beta"

## Creating a Release

**Stable release:**
```bash
git tag v1.0.0
git push origin v1.0.0
```

**Beta/prerelease:**
```bash
git tag v1.0.0-beta.1
git push origin v1.0.0-beta.1
```

## Linux Dependencies

The build requires these packages on Linux:
```
libgl1-mesa-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

## Notes

- Releases are explicit (tag-based) to avoid accidental releases from every merge
- Beta releases are marked as prereleases in GitHub
- All builds use the stable Rust toolchain via `dtolnay/rust-toolchain`
