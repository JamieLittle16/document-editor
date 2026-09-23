use std::fs::OpenOptions;
use std::io::{self, Read, Seek, SeekFrom, Write};

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn parse_u64(value: &str, name: &str) -> io::Result<u64> {
    value
        .parse::<u64>()
        .map_err(|_| invalid(format!("invalid {name}: {value}")))
}

fn parse_byte(value: &str, name: &str) -> io::Result<u8> {
    value
        .parse::<u8>()
        .map_err(|_| invalid(format!("invalid {name}: {value}")))
}

fn write_pattern(file: &mut std::fs::File, offset: u64, length: u64, byte: u8) -> io::Result<()> {
    let end = offset
        .checked_add(length)
        .ok_or_else(|| invalid("offset plus length overflow"))?;
    let file_len = file.metadata()?.len();
    if end > file_len {
        return Err(invalid(format!(
            "requested write ends at {end}, beyond backing length {file_len}"
        )));
    }

    file.seek(SeekFrom::Start(offset))?;
    let block = [byte; 8_192];
    let mut remaining = length;
    while remaining > 0 {
        let count = usize::try_from(remaining.min(block.len() as u64))
            .map_err(|_| invalid("write block length is not representable"))?;
        file.write_all(&block[..count])?;
        remaining -= u64::try_from(count).map_err(|_| invalid("write count does not fit u64"))?;
    }
    file.sync_data()
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).ok_or_else(|| invalid("missing mode"))?;
    let path = args.get(2).ok_or_else(|| invalid("missing backing path"))?;
    let offset = parse_u64(
        args.get(3).ok_or_else(|| invalid("missing offset"))?,
        "offset",
    )?;
    let length = parse_u64(
        args.get(4).ok_or_else(|| invalid("missing length"))?,
        "length",
    )?;
    let first_byte = parse_byte(args.get(5).ok_or_else(|| invalid("missing byte"))?, "byte")?;

    let mut file = OpenOptions::new().read(true).write(true).open(path)?;
    write_pattern(&mut file, offset, length, first_byte)?;

    match mode.as_str() {
        "write" => {
            println!("render_file_worker=written");
        }
        "hold-rewrite" => {
            let second_byte = parse_byte(
                args.get(6).ok_or_else(|| invalid("missing rewrite byte"))?,
                "rewrite byte",
            )?;
            println!("render_file_worker=ready");
            io::stdout().flush()?;

            let mut signal = [0_u8; 1];
            io::stdin().read_exact(&mut signal)?;
            write_pattern(&mut file, offset, length, second_byte)?;
            println!("render_file_worker=rewritten");
            io::stdout().flush()?;
        }
        other => return Err(invalid(format!("unknown mode: {other}"))),
    }

    Ok(())
}
