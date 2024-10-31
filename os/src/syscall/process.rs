//! Process management syscalls
use alloc::boxed::Box;

use crate::{
    config::{MAX_SYSCALL_NUM, PAGE_SIZE}, mm::{translated_byte_buffer, PageTable, StepByOne}, task::{
        change_program_brk, current_user_token, exit_current_and_run_next, set_curr_memory_set, suspend_current_and_run_next, TaskStatus
    }, timer::get_time_us
};
use crate::timer::get_time_ms;
use crate::task::get_curr_running_time;
use crate::task::get_curr_syscall;
use core::slice;
use core::mem;
// use crate::mm::FrameTracker;
use crate::mm::{VirtAddr};
use crate::mm::MapPermission;
use crate::task::set_unmap;
#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let res = TimeVal{
        sec:us/1_000_000,
        usec:us % 1_000_000,
    };
    //&res转换为字符指针
    let res_ptr = &res as *const TimeVal as *const u8;
    let timeval_size =mem::size_of::<TimeVal>();
    //获取用户地址
    let user_addr = translated_byte_buffer(current_user_token(),_ts as *const u8,timeval_size);
    //将res copy 到user_addr
    unsafe{
        let res_arr = slice::from_raw_parts(res_ptr,timeval_size);
        for addr in user_addr{
            addr.copy_from_slice(res_arr);
        }
    }

    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    let res=Box::new(TaskInfo{
        status:TaskStatus::Running,
        syscall_times:get_curr_syscall(),
        time:get_time_ms()-get_curr_running_time(),
    });
    let res_ptr = Box::into_raw(res) as *const TaskInfo as *const u8;
    let taskinfo_size =mem::size_of::<TaskInfo>();
    let user_addr = translated_byte_buffer(current_user_token(),_ti as *const u8,taskinfo_size);
    //将res copy 到user_addr
    unsafe{
        let res_arr = slice::from_raw_parts(res_ptr,taskinfo_size);
        for addr in user_addr{
            addr.copy_from_slice(res_arr);
        }
    }
    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");

    if port & !0x7 != 0||port & 0x7 == 0 {return -1;}
    //权限
    let map_perm = MapPermission::from_bits_truncate((port << 1) as u8)|MapPermission::U;
    
    //start对齐
    let start_va = VirtAddr::from(start);
    if start_va.page_offset()!=0 {
        return -1;}
    let end_va = VirtAddr::from(start+len);

    //检查页表是否分配
    //current_user_token()出来的页表只能用来查找，frame是空的，没有maparea
    let curr_pagetable = PageTable::from_token(current_user_token());
    let mut vpn = start_va.floor();
    for _ in 0..((len+PAGE_SIZE-1)/PAGE_SIZE){
        match curr_pagetable.translate(vpn) {
            Some(pte) =>{
                if pte.is_valid(){
                    return -1;//已经被映射
                }
            }
            _=>{}
        }
        vpn.step();
    }
    set_curr_memory_set(start_va, end_va, map_perm);
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    // println!("{}",len);
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
 //构建逻辑段
 let start_va = VirtAddr::from(start);
 //start未对齐
 if !start_va.aligned() {return -1;}
 let curr_pagetable = PageTable::from_token(current_user_token());
 let mut vpn=start_va.floor();

 for _ in 0..((len + PAGE_SIZE - 1) / PAGE_SIZE){
    match curr_pagetable.translate(vpn) {
        Some(pte) =>{
            if !pte.is_valid(){
                return -1;//未映射
            }
        }
        _=>{}
    }
    vpn.step();
}
    set_unmap(start_va, len);
    0
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
