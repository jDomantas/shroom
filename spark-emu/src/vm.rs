use std::fmt;
use std::io::{self, Read, Write};
use std::iter::FromIterator;
use std::num::Wrapping;
use instruction::Instr;

use executable::{Exe, CODE_START, DATA_START, STACK_START, STACK_SIZE};

#[derive(Debug)]
pub enum LoadError {
    BadDataLength(usize),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LoadError::BadDataLength(len) => write!(f, "data section length must be divisible by 8, but is {}", len),
        }
    }
}

#[derive(Debug)]
pub struct SmallByteSlice {
    bytes: [u8; 10],
}

impl FromIterator<u8> for SmallByteSlice {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = u8>,
    {
        let mut result = SmallByteSlice { bytes: [0; 10] };
        for (index, byte) in iter.into_iter().take(10).enumerate() {
            result.bytes[index] = byte;
        }
        result
    }
}

impl fmt::LowerHex for SmallByteSlice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut list = f.debug_list();
        for &byte in &self.bytes {
            list.entry(&format_args!("{:>02x}", byte));
        }
        list.finish()
    }
}

#[derive(Debug)]
pub enum ExecError {
    MisalignedDataAccess(u64),
    BadDataAccess(u64),
    BadCodeRead(u64),
    MisalignedStack(u64),
    InvalidInstruction(SmallByteSlice),
    Io(io::Error),
    BadDivide,
    DivByZero,
    InvalidSyscall(u64),
}

impl fmt::Display for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ExecError::MisalignedDataAccess(addr) => write!(f, "misaligned data access at {:#x}", addr),
            ExecError::BadDataAccess(addr) => write!(f, "out of range data access at {:#x}", addr),
            ExecError::BadCodeRead(addr) => write!(f, "out of range code access at {:#x}", addr),
            ExecError::MisalignedStack(sp) => write!(f, "misaligned stack with rsp = {:#x}", sp),
            ExecError::InvalidInstruction(ref bytes) => write!(f, "cannot decode instruction from {:#x}", bytes),
            ExecError::Io(ref e) => write!(f, "{}", e),
            ExecError::BadDivide => write!(f, "attempted to divide with rdx != 0"),
            ExecError::DivByZero => write!(f, "attempted to divide by 0"),
            ExecError::InvalidSyscall(id) => write!(f, "unknown syscall id: {}", id),
        }
    }
}

impl From<io::Error> for ExecError {
    fn from(err: io::Error) -> ExecError {
        ExecError::Io(err)
    }
}

pub type ExecResult<T> = Result<T, ExecError>;

#[derive(Clone)]
struct DataSection {
    start_address: u64,
    data: Vec<u64>,
}

impl DataSection {
    fn new(data: Vec<u8>) -> Result<Self, LoadError> {
        assert_eq!(STACK_START + STACK_SIZE, DATA_START);
        assert_eq!(STACK_SIZE % 8, 0);
        let mut converted_data = Vec::new();
        // zero initialize stack
        for _ in 0..(STACK_SIZE / 8) {
            converted_data.push(0u64);
        }
        if data.len() % 8 != 0 {
            return Err(LoadError::BadDataLength(data.len()));
        }
        let mut pos = 0;
        let mut curr = 0;
        let mut taken = 0;
        while pos < data.len() {
            let byte = u64::from(data[pos]);
            curr += byte << (taken * 8);
            taken += 1;
            pos += 1;
            if taken % 8 == 0 {
                converted_data.push(curr);
                taken = 0;
                curr = 0;
            }
        }
        Ok(DataSection {
            start_address: STACK_START,
            data: converted_data,
        })
    }

    fn access(&mut self, addr: u64) -> ExecResult<&mut u64> {
        if self.data.len() == 0 {
            return Err(ExecError::BadDataAccess(addr));
        }
        let last_address = self.start_address + (self.data.len() - 1) as u64 * 8;
        if addr < self.start_address || addr > last_address {
            return Err(ExecError::BadDataAccess(addr));
        }
        let addr2 = addr - self.start_address;
        if addr2 % 8 != 0 {
            return Err(ExecError::MisalignedDataAccess(addr));
        }
        Ok(&mut self.data[(addr2 / 8) as usize])
    }
}

#[derive(Clone)]
struct CodeSection {
    start_address: u64,
    data: Vec<u8>,
}

impl CodeSection {
    fn new(data: Vec<u8>) -> Self {
        CodeSection {
            start_address: CODE_START,
            data,
        }
    }

    fn load_slice(&self, addr: u64) -> ExecResult<&[u8]> {
        if addr < self.start_address || addr >= self.start_address + self.data.len() as u64 {
            return Err(ExecError::BadCodeRead(addr));
        }
        let addr2 = addr - self.start_address;
        Ok(&self.data[addr2 as usize..])
    }
}

pub struct Vm<'a> {
    rip: Wrapping<u64>,
    rax: Wrapping<u64>,
    rbx: Wrapping<u64>,
    rdx: Wrapping<u64>,
    rsp: Wrapping<u64>,
    rbp: Wrapping<u64>,
    below_flag: bool,
    zero_flag: bool,
    code: CodeSection,
    data: DataSection,
    stdin: &'a mut (Read + 'a),
    stdout: &'a mut (Write + 'a),
    have_pending_writes: bool,
    trace_instructions: bool,
}

impl<'a> Vm<'a> {
    pub fn new(
        exe: Exe,
        stdin: &'a mut (Read + 'a),
        stdout: &'a mut (Write + 'a),
        trace_instructions: bool,
    ) -> Result<Self, LoadError> {
        let code = CodeSection::new(exe.code);
        let data = DataSection::new(exe.data)?;
        Ok(Vm {
            rip: Wrapping(CODE_START),
            rax: Wrapping(0),
            rbx: Wrapping(0),
            rdx: Wrapping(0),
            rsp: Wrapping(STACK_START + STACK_SIZE),
            rbp: Wrapping(0),
            below_flag: false,
            zero_flag: false,
            code,
            data,
            stdin,
            stdout,
            have_pending_writes: false,
            trace_instructions,
        })
    }

    pub fn cycle(&mut self) -> ExecResult<()> {
        let instr = {
            let code_view = self.code.load_slice(self.rip.0)?;
            if let Some(instr) = Instr::decode(code_view) {
                instr
            } else {
                let code = code_view.iter().cloned().take(10).collect();
                return Err(ExecError::InvalidInstruction(code));
            }
        };
        self.execute_instr(instr)
    }

    fn execute_instr(&mut self, instr: Instr) -> ExecResult<()> {
        if self.trace_instructions {
            eprintln!("rip = {:#x}, instruction: {}", self.rip.0, instr);
        }
        self.rip += Wrapping(instr.len());
        match instr {
            Instr::AddRaxRbx => {
                self.rax += self.rbx;
            }
            Instr::AddRsp(value) => {
                self.rsp += Wrapping(value);
            }
            Instr::Call(offset) => {
                let return_addr = self.rip.0;
                self.push(return_addr)?;
                self.rip += Wrapping(offset);
            }
            Instr::CmpRaxRbx => {
                self.below_flag = self.rax < self.rbx;
                self.zero_flag = self.rax == self.rbx;
            }
            Instr::DivRbx => {
                if self.rdx.0 != 0 {
                    return Err(ExecError::BadDivide);
                }
                if self.rbx.0 == 0 {
                    return Err(ExecError::DivByZero);
                }
                self.rdx = self.rax % self.rbx;
                self.rax = self.rax / self.rbx;
            }
            Instr::Jmp(offset) => {
                self.rip += Wrapping(offset);
            }
            Instr::Jnz(offset) => {
                if !self.zero_flag {
                    self.rip += Wrapping(offset);
                }
            }
            Instr::Jz(offset) => {
                if self.zero_flag {
                    self.rip += Wrapping(offset);
                }
            }
            Instr::LeaRaxRbpOffset(offset) => {
                self.rax = self.rbp + Wrapping(offset);
            }
            Instr::MovRax(val) => {
                self.rax = Wrapping(val);
            }
            Instr::MovRaxOffsetRbx(offset) => {
                let addr = (self.rax + Wrapping(offset)).0;
                *self.data.access(addr)? = self.rbx.0;
            }
            Instr::MovRaxQwordRsp => {
                let value = *self.data.access(self.rsp.0)?;
                self.rax = Wrapping(value);
            }
            Instr::MovRaxRspOffset(offset) => {
                let addr = (self.rsp + Wrapping(offset)).0;
                self.rax = Wrapping(*self.data.access(addr)?);
            }
            Instr::MovRbpRsp => {
                self.rbp = self.rsp;
            }
            Instr::MovRbxRspRaxOffset(offset) => {
                let addr = (self.rsp + self.rax + Wrapping(offset)).0;
                self.rbp = Wrapping(*self.data.access(addr)?);
            }
            Instr::MovRspOffsetRbx(offset) => {
                let addr = (self.rsp + Wrapping(offset)).0;
                *self.data.access(addr)? = self.rbx.0;
            }
            Instr::MulRbx => {
                self.rax *= self.rbx;
            }
            Instr::PopRax => {
                self.rax = Wrapping(self.pop()?);
            }
            Instr::PopRbp => {
                self.rbp = Wrapping(self.pop()?);
            }
            Instr::PopRbx => {
                self.rbx = Wrapping(self.pop()?);
            }
            Instr::PopRdx => {
                self.rdx = Wrapping(self.pop()?);
            }
            Instr::PushQwordRax => {
                let value = *self.data.access(self.rax.0)?;
                self.push(value)?;
            }
            Instr::PushQwordRaxOffset(offset) => {
                let addr = (self.rax + Wrapping(offset)).0;
                let value = *self.data.access(addr)?;
                self.push(value)?;
            }
            Instr::PushRax => {
                let rax = self.rax.0;
                self.push(rax)?;
            }
            Instr::PushRbp => {
                let rbp = self.rbp.0;
                self.push(rbp)?;
            }
            Instr::PushRbx => {
                let rbx = self.rbx.0;
                self.push(rbx)?;
            }
            Instr::PushRdx => {
                let rdx = self.rdx.0;
                self.push(rdx)?;
            }
            Instr::Ret => {
                self.rip = Wrapping(self.pop()?);
            }
            Instr::SetbDl => {
                self.rdx &= Wrapping(!0xFF);
                if self.below_flag {
                    self.rdx |= Wrapping(1);
                }
            }
            Instr::SeteDl => {
                self.rdx &= Wrapping(!0xFF);
                if self.zero_flag {
                    self.rdx |= Wrapping(1);
                }
            }
            Instr::SetneDl => {
                self.rdx &= Wrapping(!0xFF);
                if !self.zero_flag {
                    self.rdx |= Wrapping(1);
                }
            }
            Instr::SubRaxRbx => {
                self.rax -= self.rbx;
            }
            Instr::SubRsp(x) => {
                self.rsp -= Wrapping(x);
            }
            Instr::TestRaxRax => {
                self.zero_flag = self.rax.0 == 0;
            }
            Instr::XorRaxRax => {
                self.rax = Wrapping(0);
            }
            Instr::XorRdxRdx => {
                self.rdx = Wrapping(0);
            }
            Instr::Syscall => {
                match self.rax.0 {
                    0 => { // exit
                        let arg = self.rbx.0;
                        ::std::process::exit(arg as i32);
                    }
                    1 => { // read_byte
                        if self.have_pending_writes {
                            self.stdout.flush()?;
                        }
                        let value = self.read_byte()?;
                        self.rbx = Wrapping(value);
                    }
                    2 => { // write_byte
                        let value = (self.rbx.0 & 0xFF) as u8;
                        self.stdout.write(&[value])?;
                        self.have_pending_writes = true;
                    }
                    other => {
                        return Err(ExecError::InvalidSyscall(other));
                    }
                }
            }
        }
        if self.rsp.0 % 8 == 0 {
            Ok(())
        } else {
            Err(ExecError::MisalignedStack(self.rsp.0))
        }
    }

    fn push(&mut self, value: u64) -> ExecResult<()> {
        self.rsp -= Wrapping(8);
        *self.data.access(self.rsp.0)? = value;
        Ok(())
    }

    fn pop(&mut self) -> ExecResult<u64> {
        let value = *self.data.access(self.rsp.0)?;
        self.rsp += Wrapping(8);
        Ok(value)
    }

    fn read_byte(&mut self) -> ExecResult<u64> {
        let mut buf = [0];
        let amount_read = self.stdin.read(&mut buf)?;
        Ok(if amount_read == 0 {
            256
        } else {
            u64::from(buf[0])
        })
    }
}
