# 1.

---

[kernel] Hello, world!
first task time is 7
[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.

程序被内核杀死

[rustsbi] RustSBI version 0.3.0-alpha.2, adapting to RISC-V SBI v1.0.0



# 2

--- ---

## 1

​	当 trap_handler 返回之后，使用 __restore 从保存在内核栈上的 Trap 上下文恢复寄存器。

​	switch切换任务上下文时恢复寄存器

## 2



sstatus

SPP 等字段给出 Trap 发生之前 CPU 处在哪个特权级（S/U）等信息

sepc

当 Trap 是一个异常的时候，记录 Trap 发生之前执行的最后一条指令的地址

sscratch

指向用户栈的地址，trap时与内核栈地址中转交换

## 3

x2在用户栈切换之后保存 x4无需保存

## 4

L60 sp为内核 sscratch为用户栈

## 5

sret之后使用 sscratch的值返回用户栈

## 6

L13 sp为内核 sscratch为用户栈

## 7

U态进入S态是csrrw sp, sscratch, sp这条指令上

# 总结报告

在run_first_task和run_next_trask中使用api记录了运行时间，在task context中设置数组记录syscall次数。

# 荣誉准则



1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

   > *与多位群友交流gdb调试与思路,向chatgpt询问关于rust语法问题*

2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

   > *v3版本文档*

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。

# 建议

可能是我水平不够，文档看不懂，希望多加一些图。
