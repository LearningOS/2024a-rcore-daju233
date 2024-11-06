//! File and filesystem-related syscalls

// use core::{mem, slice};

// use alloc::sync::Arc;
// use easy_fs::Inode;

use core::{mem, slice};

use crate::fs::{open_file, OpenFlags, Stat, StatMode, ROOT_INODE};
use crate::mm::{translated_byte_buffer, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_write", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_read", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    trace!("kernel:pid[{}] sys_open", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        // if inode.inner.exclusive_access().inode.get_information().1==0{
        //     inode.inner.exclusive_access().inode.get_information().1+=1;
        // }
        // println!("{:?}",inode.inner.exclusive_access().inode.get_information());
        // if inode.inner.exclusive_access().inode.get_information().1==0{
        //     return -1;
        // }
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        println!("{}",fd);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    trace!("kernel:pid[{}] sys_close", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

/// YOUR JOB: Implement linkat.
pub fn sys_linkat(old_name: *const u8, new_name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_linkat NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let token = current_user_token();
    let old_path = translated_str(token, old_name);
    let new_path = translated_str(token, new_name);
    if old_path==new_path{
        return -1;
    }
    ROOT_INODE.my_create(new_path.as_str(),old_path.as_str());
    0
}

/// YOUR JOB: Implement unlinkat.
pub fn sys_unlinkat(name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_unlinkat NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let token = current_user_token();
    let path = translated_str(token, name);
    ROOT_INODE.my_unlink(path.as_str());
    0
}
/// YOUR JOB: Implement fstat.
pub fn sys_fstat(fd: usize, st: *mut Stat) -> isize {
    trace!(
        "kernel:pid[{}] sys_fstat NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let task =current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let fd_table = inner.fd_table.clone();
    drop(inner);
    let os_inode = fd_table[fd].clone().unwrap();
    let (mode,nlink,ino) = os_inode.get_nlink_and_type_and_inode();
    let statmode:StatMode = match mode {
        0o040000 => StatMode::DIR,
        0o100000 => StatMode::FILE,
        _ =>StatMode::NULL,
    };
    let res = Stat{
        dev:0,
        mode:statmode,
        nlink,
        ino: ino.into(),
        pad:[0;7],
    };


    let stat_size = mem::size_of::<Stat>();
    let user_addr = translated_byte_buffer(current_user_token(),st as *const u8,stat_size);
    let res_ptr = &res as *const Stat as *const u8;
    unsafe {      
        let res_arr = slice::from_raw_parts(res_ptr,stat_size);
        for addr in user_addr{
        addr.copy_from_slice(res_arr);
        }}
    0
}
