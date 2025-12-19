set shell := ["powershell", "-NoProfile", "-Command"]

test:
    cargo test

build:
    cargo build --release
    New-Item -ItemType Directory -Force dist | Out-Null
    Copy-Item -Force target\release\MagMerge.exe dist
    Copy-Item -Force target\release\magmerge_cli.exe dist

dist: build
