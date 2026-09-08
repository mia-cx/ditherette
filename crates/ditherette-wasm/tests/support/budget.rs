/// Find the mandatory cold execution boundary independently of optional retention.
/// This is a capacity search, not a timing measurement; every attempt owns a fresh instance.
pub fn minimum(upper: u64, mut succeeds: impl FnMut(u64) -> bool) -> u64 {
    assert!(succeeds(upper));
    let (mut low, mut high) = (0, upper);
    while low < high {
        let middle = low + (high - low) / 2;
        if succeeds(middle) {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    low
}
