export def test [] {
  cargo test -F mockall
}

export def lint [] {
  cargo fmt
  cargo clippy
}