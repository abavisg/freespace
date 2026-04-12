use std::io::{self, Write};

pub enum OutputFormat {
    Table,
    Json,
}

impl OutputFormat {
    pub fn from_flag(json: bool) -> Self {
        if json {
            OutputFormat::Json
        } else {
            OutputFormat::Table
        }
    }
}

pub fn write_json<T: serde::Serialize>(value: &T) -> anyhow::Result<()> {
    let stdout = io::stdout();
    serde_json::to_writer(stdout.lock(), value)?;
    writeln!(io::stdout().lock())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_flag_json() {
        assert!(matches!(OutputFormat::from_flag(true), OutputFormat::Json));
    }

    #[test]
    fn from_flag_table() {
        assert!(matches!(OutputFormat::from_flag(false), OutputFormat::Table));
    }

    #[test]
    fn write_json_serializes_struct() {
        // Test that write_json accepts a serde-serializable value without panicking.
        // We can't capture stdout in a unit test easily, so verify it returns Ok.
        #[derive(serde::Serialize)]
        struct TestValue { status: &'static str }
        let result = write_json(&TestValue { status: "test" });
        assert!(result.is_ok(), "write_json must succeed for a valid serializable value");
    }
}
