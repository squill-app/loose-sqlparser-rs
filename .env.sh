# -----------------------------------------------------------------------------
# Run code coverage
# -----------------------------------------------------------------------------
function codecov() {
  export LLVM_PROFILE_FILE=rsql-%p-%m.profraw
  export RUST_BACKTRACE=1
  export RUST_LOG="info"
  export RUST_LOG_SPAN_EVENTS=full
  export RUSTFLAGS=-Cinstrument-coverage
  export RUSTDOCFLAGS=-Cinstrument-coverage

  clean_codecov

  cargo test && \
  grcov $(find . -name "rsql-*.profraw" -print) \
    -s . \
    --branch \
    --ignore-not-existing \
    --ignore='target/*' \
    --ignore='examples/*' \
    --ignore='/*' \
    --binary-path ./target/debug/ \
    -t html \
    -o ./target/coverage/

  if [ -n "$1" ]; then
    local arg="$1"
    case $arg in
      "--open")
        open ./target/coverage/index.html
        ;;

      *)
        echo "Usage: codecov [--open]"
        ;;
    esac
  fi
  # Unset environment variables
  unset LLVM_PROFILE_FILE
  unset RUST_BACKTRACE
  unset RUST_LOG
  unset RUST_LOG_SPAN_EVENTS
  unset RUSTFLAGS
  unset RUSTDOCFLAGS
}

function clean_codecov {
  local profile_files=($(find . -name "rsql-*.profraw" -print))
  for file in "${profile_files[@]}"; do
    rm $file
  done
  [ -f "lcov.info" ] && rm lcov.info
  rm -rf ./target/coverage
}

# -----------------------------------------------------------------------------
# Clean code coverage and cargo files & reset the environment
# -----------------------------------------------------------------------------
function clean() {
  clean_codecov
  cargo clean
}
