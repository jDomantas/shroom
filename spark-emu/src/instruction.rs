use std::fmt;

#[derive(Debug, Copy, Clone)]
pub enum Instr {
    PopRax,
    PopRbx,
    PopRbp,
    PopRdx,
    PushRax,
    PushRbx,
    PushRbp,
    PushRdx,
    AddRaxRbx,
    SubRaxRbx,
    MulRbx,
    DivRbx,
    PushQwordRax,
    PushQwordRaxOffset(u64),
    MovRaxRspOffset(u64),
    MovRaxOffsetRbx(u64),
    AddRsp(u64),
    SubRsp(u64),
    CmpRaxRbx,
    SeteDl,
    XorRaxRax,
    XorRdxRdx,
    SetneDl,
    SetbDl,
    MovRax(u64),
    TestRaxRax,
    Call(u64),
    Jmp(u64),
    Jnz(u64),
    Jz(u64),
    Ret,
    LeaRaxRbpOffset(u64),
    MovRbxRspRaxOffset(u64),
    MovRspOffsetRbx(u64),
    MovRaxQwordRsp,
    MovRbpRsp,
    Syscall,
}

impl Instr {
    pub fn len(&self) -> u64 {
        match *self {
            Instr::PopRax => 1,
            Instr::PopRbx => 1,
            Instr::PopRbp => 1,
            Instr::PopRdx => 1,
            Instr::PushRax => 1,
            Instr::PushRbx => 1,
            Instr::PushRbp => 1,
            Instr::PushRdx => 1,
            Instr::AddRaxRbx => 3,
            Instr::SubRaxRbx => 3,
            Instr::MulRbx => 3,
            Instr::DivRbx => 3,
            Instr::PushQwordRax => 2,
            Instr::PushQwordRaxOffset(_) => 6,
            Instr::MovRaxRspOffset(_) => 8,
            Instr::MovRaxOffsetRbx(_) => 7,
            Instr::AddRsp(_) => 7,
            Instr::SubRsp(_) => 7,
            Instr::CmpRaxRbx => 3,
            Instr::SeteDl => 3,
            Instr::XorRaxRax => 3,
            Instr::XorRdxRdx => 3,
            Instr::SetneDl => 3,
            Instr::SetbDl => 3,
            Instr::MovRax(_) => 10,
            Instr::TestRaxRax => 3,
            Instr::Call(_) => 5,
            Instr::Jmp(_) => 5,
            Instr::Jnz(_) => 6,
            Instr::Jz(_) => 6,
            Instr::Ret => 1,
            Instr::LeaRaxRbpOffset(_) => 7,
            Instr::MovRbxRspRaxOffset(_) => 8,
            Instr::MovRspOffsetRbx(_) => 8,
            Instr::MovRaxQwordRsp => 4,
            Instr::MovRbpRsp => 3,
            Instr::Syscall => 2,
        }
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() >= 1 {
            match bytes[0] {
                0x58 => return Some(Instr::PopRax),
                0x5B => return Some(Instr::PopRbx),
                0x5D => return Some(Instr::PopRbp),
                0x5A => return Some(Instr::PopRdx),
                0x50 => return Some(Instr::PushRax),
                0x53 => return Some(Instr::PushRbx),
                0x55 => return Some(Instr::PushRbp),
                0x52 => return Some(Instr::PushRdx),
                0xC3 => return Some(Instr::Ret),
                _ => {}
            }
        }
        if bytes.len() >= 2 {
            match (bytes[0], bytes[1]) {
                (0xFF, 0x30) => return Some(Instr::PushQwordRax),
                (0x0F, 0x05) => return Some(Instr::Syscall),
                _ => {}
            }
        }
        if bytes.len() >= 3 {
            match (bytes[0], bytes[1], bytes[2]) {
                (0x48, 0x01, 0xD8) => return Some(Instr::AddRaxRbx),
                (0x48, 0x29, 0xD8) => return Some(Instr::SubRaxRbx),
                (0x48, 0xF7, 0xE3) => return Some(Instr::MulRbx),
                (0x48, 0xF7, 0xF3) => return Some(Instr::DivRbx),
                (0x48, 0x39, 0xD8) => return Some(Instr::CmpRaxRbx),
                (0x0F, 0x94, 0xC2) => return Some(Instr::SeteDl),
                (0x48, 0x31, 0xC0) => return Some(Instr::XorRaxRax),
                (0x48, 0x31, 0xD2) => return Some(Instr::XorRdxRdx),
                (0x0F, 0x95, 0xC2) => return Some(Instr::SetneDl),
                (0x0F, 0x92, 0xC2) => return Some(Instr::SetbDl),
                (0x48, 0x85, 0xC0) => return Some(Instr::TestRaxRax),
                (0x48, 0x89, 0xE5) => return Some(Instr::MovRbpRsp),
                _ => {}
            }
        }
        if bytes.len() >= 4 {
            match (bytes[0], bytes[1], bytes[2], bytes[3]) {
                (0x48, 0x8B, 0x04, 0x24) => return Some(Instr::MovRaxQwordRsp),
                _ => {}
            }
        }
        if bytes.len() >= 5 {
            match (bytes[0], bytes[1], bytes[2], bytes[3], bytes[4]) {
                (0xE8, a0, a1, a2, a3) => {
                    let arg = four_byte_sign_extend(a3, a2, a1, a0);
                    return Some(Instr::Call(arg));
                }
                (0xE9, a0, a1, a2, a3) => {
                    let arg = four_byte_sign_extend(a3, a2, a1, a0);
                    return Some(Instr::Jmp(arg));
                }
                _ => {}
            }
        }
        if bytes.len() >= 6 {
            match (bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]) {
                (0xFF, 0xB0, a0, a1, a2, a3) => {
                    let arg = four_byte_sign_extend(a3, a2, a1, a0);
                    return Some(Instr::PushQwordRaxOffset(arg));
                }
                (0x0F, 0x85, a0, a1, a2, a3) => {
                    let arg = four_byte_sign_extend(a3, a2, a1, a0);
                    return Some(Instr::Jnz(arg));
                }
                (0x0F, 0x84, a0, a1, a2, a3) => {
                    let arg = four_byte_sign_extend(a3, a2, a1, a0);
                    return Some(Instr::Jz(arg));
                }
                _ => {}
            }
        }
        if bytes.len() >= 7 {
            match (bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6]) {
                (0x48, 0x89, 0x98, a0, a1, a2, a3) => {
                    let arg = four_byte_sign_extend(a3, a2, a1, a0);
                    return Some(Instr::MovRaxOffsetRbx(arg));
                }
                (0x48, 0x81, 0xC4, a0, a1, a2, a3) => {
                    let arg = four_byte_sign_extend(a3, a2, a1, a0);
                    return Some(Instr::AddRsp(arg));
                }
                (0x48, 0x81, 0xEC, a0, a1, a2, a3) => {
                    let arg = four_byte_sign_extend(a3, a2, a1, a0);
                    return Some(Instr::SubRsp(arg));
                }
                (0x48, 0x8D, 0x85, a0, a1, a2, a3) => {
                    let arg = four_byte_sign_extend(a3, a2, a1, a0);
                    return Some(Instr::LeaRaxRbpOffset(arg));
                }
                _ => {}
            }
        }
        if bytes.len() >= 8 {
            match (bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]) {
                (0x48, 0x8B, 0x84, 0x24, a0, a1, a2, a3) => {
                    let arg = four_byte_sign_extend(a3, a2, a1, a0);
                    return Some(Instr::MovRaxRspOffset(arg));
                }
                (0x48, 0x8B, 0x9C, 0x04, a0, a1, a2, a3) => {
                    let arg = four_byte_sign_extend(a3, a2, a1, a0);
                    return Some(Instr::MovRbxRspRaxOffset(arg));
                }
                (0x48, 0x89, 0x9C, 0x24, a0, a1, a2, a3) => {
                    let arg = four_byte_sign_extend(a3, a2, a1, a0);
                    return Some(Instr::MovRspOffsetRbx(arg));
                }
                _ => {}
            }
        }
        if bytes.len() >= 10 {
            match (bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9]) {
                (0x48, 0xB8, a0, a1, a2, a3, a4, a5, a6, a7) => {
                    let arg = eight_byte(a7, a6, a5, a4, a3, a2, a1, a0);
                    return Some(Instr::MovRax(arg));
                }
                _ => {}
            }
        }
        None
    }
}

impl fmt::Display for Instr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Instr::PopRax => write!(f, "pop rax"),
            Instr::PopRbx => write!(f, "pop rbx"),
            Instr::PopRbp => write!(f, "pop rbp"),
            Instr::PopRdx => write!(f, "pop rdx"),
            Instr::PushRax => write!(f, "push rax"),
            Instr::PushRbp => write!(f, "push rbp"),
            Instr::PushRbx => write!(f, "push rbx"),
            Instr::PushRdx => write!(f, "push rdx"),
            Instr::AddRaxRbx => write!(f, "add rax, rbx"),
            Instr::SubRaxRbx => write!(f, "sub rax, rbx"),
            Instr::MulRbx => write!(f, "mul rbx"),
            Instr::DivRbx => write!(f, "div rbx"),
            Instr::PushQwordRax => write!(f, "push qword [rax]"),
            Instr::PushQwordRaxOffset(o) => write!(f, "push qword [rax + {}]", o),
            Instr::MovRaxRspOffset(o) => write!(f, "mov rax, [rsp + {}]", o),
            Instr::MovRaxOffsetRbx(o) => write!(f, "mov [rax + {}], rbx", o),
            Instr::AddRsp(x) => write!(f, "add rsp, {}", x),
            Instr::SubRsp(x) => write!(f, "sub rsp, {}", x),
            Instr::CmpRaxRbx => write!(f, "cmp rax, rbx"),
            Instr::SeteDl => write!(f, "sete dl"),
            Instr::XorRaxRax => write!(f, "xor rax, rax"),
            Instr::XorRdxRdx => write!(f, "xor rdx, rdx"),
            Instr::SetneDl => write!(f, "setne dl"),
            Instr::SetbDl => write!(f, "setb dl"),
            Instr::MovRax(x) => write!(f, "mov rax, {}", x),
            Instr::TestRaxRax => write!(f, "test rax, rax"),
            Instr::Call(off) => write!(f, "call {}", off),
            Instr::Jmp(off) => write!(f, "jmp {}", off),
            Instr::Jnz(off) => write!(f, "jnz {}", off),
            Instr::Jz(off) => write!(f, "jz {}", off),
            Instr::Ret => write!(f, "ret"),
            Instr::LeaRaxRbpOffset(o) => write!(f, "lea rax, [rbp + {}]", o),
            Instr::MovRbxRspRaxOffset(o) => write!(f, "mov rbx, [rsp + rax + {}]", o),
            Instr::MovRspOffsetRbx(o) => write!(f, "mov [rsp + {}], rbx", o),
            Instr::MovRaxQwordRsp => write!(f, "mov rax, [rsp]"),
            Instr::MovRbpRsp => write!(f, "mov rbp, rsp"),
            Instr::Syscall => write!(f, "syscall"),
        }
    }
}

fn four_byte_sign_extend(a3: u8, a2: u8, a1: u8, a0: u8) -> u64 {
    let a3 = u64::from(a3) << 24;
    let a2 = u64::from(a2) << 16;
    let a1 = u64::from(a1) << 8;
    let a0 = u64::from(a0) << 0;
    let total = a3 | a2 | a1 | a0;
    if total & (1 << 31) == 0 {
        total
    } else {
        total | 0xFFFFFFFF00000000
    }
}

fn eight_byte(a7: u8, a6: u8, a5: u8, a4: u8, a3: u8, a2: u8, a1: u8, a0: u8) -> u64 {
    let a7 = u64::from(a7) << 56;
    let a6 = u64::from(a6) << 48;
    let a5 = u64::from(a5) << 40;
    let a4 = u64::from(a4) << 32;
    let a3 = u64::from(a3) << 24;
    let a2 = u64::from(a2) << 16;
    let a1 = u64::from(a1) << 8;
    let a0 = u64::from(a0) << 0;
    a7 | a6 | a5 | a4 | a3 | a2 | a1 | a0
}
