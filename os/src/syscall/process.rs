//! Process management syscalls
//!
use alloc::sync::Arc;

use alloc::boxed::Box;

use crate::{
    config::MAX_SYSCALL_NUM, fs::{open_file, OpenFlags}, mm::{translated_refmut, translated_str}, task::{
        add_task, current_task, current_user_token, exit_current_and_run_next, get_syscall, get_task_running_time, set_memory_set, set_unmap, suspend_current_and_run_next, TaskStatus 
    }
};
use crate::timer::get_time_ms;
use core::slice;
use core::mem;

// use crate::{
//     config::MAX_SYSCALL_NUM, loader::get_app_data_by_name, mm::{translated_refmut, translated_str}, task::{
//         add_task, current_task, current_user_token, exit_current_and_run_next, get_syscall, get_task_running_time, set_memory_set, set_unmap, suspend_current_and_run_next, TaskStatus, 
//     }
// };

use crate::mm::MapPermission;
use crate::timer::get_time_us;
use crate::mm::translated_byte_buffer;
use crate::mm::VirtAddr;
use crate::mm::PageTable;
use crate::config::PAGE_SIZE;
use crate::mm::StepByOne;
// use crate::mm::FrameTracker;
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
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        task.exec(all_data.as_slice());
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!("kernel::pid[{}] sys_waitpid [{}]", current_task().unwrap().pid.0, pid);
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_get_time NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
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
    trace!(
        "kernel:pid[{}] sys_task_info NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let res=Box::new(TaskInfo{
        status:TaskStatus::Running,
        syscall_times:get_syscall(),
        time:get_time_ms()-get_task_running_time(),
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

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_mmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );

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
    set_memory_set(start_va, end_va, map_perm);
    0
}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_munmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
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
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let token = current_user_token();
    let path = translated_str(token, path);
    let current_task = current_task().unwrap();
    let mut new_pid:isize = -1;
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY){
        let all_data = app_inode.read_all();
        let new_task = current_task.task_spawn(&all_data);
    // let new_task = current_task.task_spawn(get_app_data_by_name(path.as_str()).unwrap());
    new_pid = new_task.pid.0 as isize;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    }
    new_pid as isize
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    if prio<2 {return -1;}
    let current_task = current_task().unwrap();
    current_task.inner_exclusive_access().prioity = prio as usize; 
    prio
}
