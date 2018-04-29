#[macro_use]
extern crate structopt;

mod executable;
mod instruction;
mod vm;

use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "spark-emu")]
struct Opt {
    /// Trace executed instructions
    #[structopt(short = "t", long = "trace")]
    trace: bool,
    /// Path to spark executable
    #[structopt(parse(from_os_str))]
    file: PathBuf,
    /// File to be used as program's stdin
    #[structopt(short = "i", long = "stdin", parse(from_os_str))]
    stdin: Option<PathBuf>,
    /// File to be used as program's stdout
    #[structopt(short = "o", long = "stdout", parse(from_os_str))]
    stdout: Option<PathBuf>,
}

#[derive(Debug)]
enum Error {
    ExeRead(executable::ReadError),
    VmLoad(vm::LoadError),
    Exec(vm::ExecError),
    Io(io::Error),
}

impl From<executable::ReadError> for Error {
    fn from(err: executable::ReadError) -> Error {
        Error::ExeRead(err)
    }
}

impl From<vm::LoadError> for Error {
    fn from(err: vm::LoadError) -> Error {
        Error::VmLoad(err)
    }
}

impl From<vm::ExecError> for Error {
    fn from(err: vm::ExecError) -> Error {
        Error::Exec(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::ExeRead(ref e) => write!(f, "{}", e),
            Error::VmLoad(ref e) => write!(f, "{}", e),
            Error::Exec(ref e) => write!(f, "{}", e),
            Error::Io(ref e) => write!(f, "{}", e),
        }
    }
}

fn run() -> Result<(), Error> {
    let opt = Opt::from_args();
    let exe = executable::Exe::read_from_file(&opt.file)?;

    let (stdin, stdout);
    let mut input: Box<Read> = if let Some(path) = opt.stdin {
        Box::new(fs::File::open(path)?)
    } else {
        stdin = io::stdin();
        Box::new(stdin.lock())
    };
    let mut output: Box<Write> = if let Some(path) = opt.stdout {
        Box::new(fs::File::create(path)?)
    } else {
        stdout = io::stdout();
        Box::new(stdout.lock())
    };

    let mut vm = vm::Vm::new(exe, input.as_mut(), output.as_mut(), opt.trace)?;
    loop {
        vm.cycle()?;
    }
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
