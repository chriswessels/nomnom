use vergen::EmitBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    EmitBuilder::builder()
        .build_timestamp()
        .git_sha(false)
        .git_describe(true, true, None)
        .emit()?;
    Ok(())
}
