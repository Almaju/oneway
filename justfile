# Oneway Language — Development Commands

set quiet

default:
    @just --list

# Build the compiler
build:
    cargo build

# Build in release mode
release:
    cargo build --release

# Run all tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Run an .ow file (compile + execute)
run file:
    #!/usr/bin/env sh
    set -e
    cargo build --quiet 2>/dev/null
    base="$(echo '{{file}}' | sed 's/\.ow$//')"
    rs="${base}.rs"
    cargo run --quiet -- --compile '{{file}}' 2>/dev/null
    "./${base}"
    rm -f "${rs}" "${base}"

# Emit generated Rust code for an .ow file
emit file:
    cargo run --quiet -- --emit-rust {{file}} 2>/dev/null

# Check sort order of an .ow file
check file:
    cargo run --quiet -- --check {{file}} 2>/dev/null

# Show tokens for an .ow file
tokens file:
    cargo run --quiet -- --tokens {{file}} 2>/dev/null

# Show AST for an .ow file
ast file:
    cargo run --quiet -- --ast {{file}} 2>/dev/null

# Compile an .ow file to binary (no run)
compile file:
    cargo run --quiet -- --compile {{file}} 2>/dev/null

# Run all examples (continues on failure)
examples: build
    #!/usr/bin/env sh
    pass=0; fail=0; skip=0
    for f in examples/*.ow; do
        name=$(basename "$f" .ow)
        base="examples/${name}"
        printf "%-20s" "$name"
        if cargo run --quiet -- --compile "$f" 2>/dev/null; then
            output=$("./${base}" 2>&1) && {
                echo "✓  $output"
                pass=$((pass + 1))
            } || {
                echo "✗  (runtime error)"
                fail=$((fail + 1))
            }
            rm -f "${base}" "${base}.rs"
        else
            echo "·  (skip — does not compile yet)"
            skip=$((skip + 1))
            rm -f "${base}.rs"
        fi
    done
    echo ""
    echo "${pass} passed, ${fail} failed, ${skip} skipped"

# Emit Rust for all examples
emit-all: build
    #!/usr/bin/env sh
    for f in examples/*.ow; do
        name=$(basename "$f" .ow)
        echo "=== $name ==="
        cargo run --quiet -- --emit-rust "$f" 2>/dev/null || echo "(failed to emit)"
        echo ""
    done

# Check all examples for sort order
check-all: build
    #!/usr/bin/env sh
    for f in examples/*.ow; do
        name=$(basename "$f" .ow)
        printf "%-20s" "$name"
        if cargo run --quiet -- --check "$f" 2>/dev/null; then
            echo "✓"
        else
            echo "✗"
        fi
    done

# Build the LSP server
lsp:
    cargo build --bin oneway-lsp --release

# Install the LSP server to ~/.cargo/bin
install-lsp: lsp
    cp target/release/oneway-lsp ~/.cargo/bin/oneway-lsp
    @echo "Installed oneway-lsp to ~/.cargo/bin/oneway-lsp"
    @echo ""
    @echo "Add to Zed settings.json:"
    @echo '  "lsp": {'
    @echo '    "oneway-lsp": {'
    @echo '      "binary": { "path": "oneway-lsp" }'
    @echo '    }'
    @echo '  },'
    @echo '  "languages": {'
    @echo '    "Oneway": {'
    @echo '      "language_servers": ["oneway-lsp"]'
    @echo '    }'
    @echo '  }'

# Format compiler source
fmt:
    cargo fmt

# Lint compiler source
clippy:
    cargo clippy -- -W warnings

# Clean build artifacts + compiled examples
clean:
    cargo clean
    rm -f examples/*.rs
    find examples -maxdepth 1 -type f ! -name '*.ow' -delete
