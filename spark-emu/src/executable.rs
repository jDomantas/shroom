use std::fmt;
use std::fs;
use std::io::{self, prelude::*};
use std::path::Path;

pub const CODE_START: u64 = 1024 * 1024 * 256;
pub const DATA_START: u64 = 1024 * 1024 * 512;
pub const STACK_START: u64 = 1024 * 1024 * 511;
pub const STACK_SIZE: u64 = 1024 * 1024;

const MAGIC_STRING: [u8; 8] = *b"sparkexe";
const MAX_CODE_LENTGH: u64 = 255 * 1024 * 1024; // 255 MB
const MAX_DATA_LENGTH: u64 = (1024 + 512) * 1024 * 1024; // 1.5 GB

#[derive(Debug)]
pub enum ReadError {
    Io(io::Error),
    BadHeader,
    BadLength,
    CodeTooLong,
    DataTooLong,
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ReadError::Io(ref e) => write!(f, "{}", e),
            ReadError::BadHeader => write!(f, "bad program header"),
            ReadError::BadLength => write!(f, "file is shorter than length in the header"),
            ReadError::CodeTooLong => write!(f, "code section is too long"),
            ReadError::DataTooLong => write!(f, "data section is too long"),
        }
    }
}

impl From<io::Error> for ReadError {
    fn from(err: io::Error) -> Self {
        ReadError::Io(err)
    }
}

#[derive(Clone)]
pub struct Exe {
    pub code: Vec<u8>,
    pub data: Vec<u8>,
}

fn format_amount(bytes: usize) -> impl fmt::Debug {
    struct Helper { bytes: usize }
    impl fmt::Debug for Helper {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            if self.bytes % 10 == 1 && self.bytes % 100 != 11 {
                write!(f, "[ <{} byte> ]", self.bytes)
            } else {
                write!(f, "[ <{} bytes> ]", self.bytes)
            }
        }
    }
    Helper { bytes }
}

impl fmt::Debug for Exe {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Exe")
            .field("code", &format_amount(self.code.len()))
            .field("data", &format_amount(self.data.len()))
            .finish()
    }
}

impl Exe {
    pub fn read_from_file<P: AsRef<Path>>(path: P) -> Result<Exe, ReadError> {
        let mut file = fs::File::open(path)?;
        let magic_string = read_quad_word(&mut file)?;
        if magic_string != MAGIC_STRING {
            return Err(ReadError::BadHeader);
        }
        let code_length = read_u64(&mut file)?;
        let data_length = read_u64(&mut file)?;
        if code_length > MAX_CODE_LENTGH {
            return Err(ReadError::CodeTooLong);
        }
        if data_length > MAX_DATA_LENGTH {
            return Err(ReadError::DataTooLong);
        }
        let mut code = vec![0; code_length as usize];
        let mut data = vec![0; data_length as usize];
        file.read_exact(&mut code).map_err(convert_unexpected_eof)?;
        file.read_exact(&mut data).map_err(convert_unexpected_eof)?;
        Ok(Exe { code, data })
    }
}

fn convert_unexpected_eof(err: io::Error) -> ReadError {
    if err.kind() == io::ErrorKind::UnexpectedEof {
        ReadError::BadLength
    } else {
        ReadError::Io(err)
    }
}

fn read_quad_word<R: Read>(mut reader: R) -> Result<[u8; 8], ReadError> {
    let mut buf = [0; 8];
    match reader.read_exact(&mut buf) {
        Ok(_) => Ok(buf),
        Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => Err(ReadError::BadHeader),
        Err(e) => Err(ReadError::Io(e)),
    }
}

fn read_u64<R: Read>(reader: R) -> Result<u64, ReadError> {
    read_quad_word(reader)
        .map(|buf| {
            let mut total = 0;
            for &byte in buf.iter().rev() {
                total = total * 256 + u64::from(byte);
            }
            total
        })
}
