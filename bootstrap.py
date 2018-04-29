import os
import sys
import subprocess

def get_temp_path(source_path):
    return os.path.splitext(source_path)[0] + '.rs'

def ident_line(original, new):
    new = new.strip()
    i = 0
    while i < len(original) and original[i] == ' ':
        i = i + 1
    return ' ' * i + new

statics = set()

def is_ident_char(ch):
    return ch in '1234567890_qwertyuiopasdfghjklzxcvbnmQWERTYUIOPASDFGHJKLZXCVBNM'

def translate_line(line):
    original = line
    line = line.strip()
    if line.startswith('struct '):
        return '#[derive(Debug, Copy, Clone)]\n' + line
    line = (line
        .replace('int ', 'usize ')
        .replace(' int', ' usize')
        .replace('int;', 'usize;')
        .replace('let ', 'let mut ')
        .replace('type', 'type_')
        .replace('struct', 'struct_')
        .replace('static ', 'static mut ')
        .replace('fn ', 'unsafe fn ')
        .replace('main()', 'main_()'))
    if '//' in line:
        for i in range(len(line)):
            if line[i:i+2] == '//':
                line = line[:i].strip()
                break
    for name in statics:
        if name not in line:
            continue
        replaced = '(*' + name + '.as_mut().unwrap())'
        i = 0
        while i <= len(line) - len(name):
            if i > 0 and is_ident_char(line[i - 1]):
                i = i + 1
                continue
            if i + len(name) < len(line) and is_ident_char(line[i + len(name)]):
                i = i + 1
                continue
            if line[i:i + len(name)] == name:
                line = line[:i] + replaced + line[i + len(name):]
                i = i + len(replaced)
                continue
            i = i + 1
    if line.startswith('static '):
        parts = line.split()
        name = parts[2][:-1]
        typ = ' '.join(parts[3:])[:-1]
        statics.add(name)
        line = 'static mut ' + name + ': Option<' + typ + '> = None;'
    elif line.startswith('let ') and '=' not in line:
        line = line[:-1] + ' = unsafe { std::mem::zeroed() };'
    elif line.startswith('if '):
        line = 'if (' + line[3:-2] + ') as usize != 0 {'
    elif line.startswith('unsafe fn') and '()' not in line:
        line = line.replace('(', '(mut ').replace(', ', ', mut ') # ))

    return ident_line(original, line)

def translate(source):
    prelude = '''

fn __syscall_exit(code: usize) {
    println!("Exit code: {}", code);
    std::process::exit(0);
}

unsafe fn __syscall_read() -> usize {
    let mut buf = [0];
    let amount_read = FILE_READER.as_mut().unwrap().read(&mut buf).expect("failed read");
    if amount_read == 0 {
        256
    } else {
        usize::from(buf[0])
    }
}

unsafe fn __syscall_write(byte: usize) {
    let buf = [byte as u8];
    let writer = FILE_WRITER.as_mut().unwrap();
    writer.write_all(&buf).expect("failed to write");
    writer.flush().expect("failed to flush");
}

use std::fs;
use std::io::prelude::*;

static mut FILE_READER: Option<fs::File> = None;
static mut FILE_WRITER: Option<fs::File> = None;

fn main() {
    unsafe {
        FILE_READER = Some(fs::File::open("test.txt").expect("failed to open source file"));
        FILE_WRITER = Some(fs::File::create("out.bin").expect("failed to create output file"));
        init_statics();
        main_();
    }
}

unsafe fn init_statics() {
'''
    disable_lints = '#![allow(unused_mut, unused_unsafe, unused_parens, bad_style, dead_code, unused_variables)]\n'
    result = []
    for line in source.splitlines():
        result.append(translate_line(line))
    main_code = '\n'.join(result)
    for name in statics:
        prelude += '    {} = Some(std::mem::zeroed());\n'.format(name)
    prelude += '}\n'
    return disable_lints + main_code + prelude

def compile(source_path):
    temp_path = get_temp_path(source_path)
    with open(source_path) as f:
        source = f.read()
    source = translate(source)
    with open(temp_path, 'w') as f:
        f.write(source)
    subprocess.run(['rustc', '-O', temp_path])

if __name__ == '__main__':
    args = sys.argv
    if len(args) != 2:
        print('usage: python {} <file>'.format(args[0]))
    else:
        compile(args[1])
