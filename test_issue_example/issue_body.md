This is a test issue created by `create_issue.sh` to verify the auto-reporting flow.

### What happened?

Test crash output:
```
thread 'main' panicked at src/example.rs:42:5:
assertion `left == right` failed
  left: 123
 right: 456
```

### Reproducer

```rust
fn main() {
    assert_eq!(123, 456);
}
```

Please close this issue - it was created for testing purposes only.

[compressed.zip - attach manually after creating issue]
