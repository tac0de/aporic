// Illustrative Rust state model; this file is not a built product.
struct Record { tenant: u64, version: u64, value: u64 }
impl Record {
    fn update(&mut self, tenant: u64, expected: u64, value: u64) -> Result<(), &'static str> {
        if tenant != self.tenant { return Err("permission denied"); }
        if expected != self.version { return Err("version conflict"); }
        self.value = value; self.version += 1; Ok(())
    }
}
