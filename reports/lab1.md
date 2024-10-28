# 第三章报告

## 实验实现的功能

1. 在`task.rs`中的结构体`TaskControlBlock`中添加两个成员`start_time`和`syscall_times`,分别代表这一任务的启动时间和任务所进行的系统调用及调用次数。
2. 在`src/task/mod.rs`中`TASK_MANAGER`的初始化过程中添加`start_time`和`syscall_times`，并设置初始值为0。
3. 为`TaskManager`结构体添加一个方法，根据传入的syscall的ID，将`syscall_times`数组中的相对应项加1。然后将这一方法包装成一个公共函数，并`src/syscall/mod.rs`的`syscall`中调用这一函数。这就保证任务每次进行的系统调用都会被记录。
4. 在`run_first_task`和`run_next_task`中，调用`timer::get_time_us`获取任务首次启动的时间并转换为ms，存储在本任务的`start_time`项中。
5. 为`TaskManager`添加两个方法，分别获取当前任务的`start_time`和`syscall_times`，并将这两个方法包装为两个公共函数`get_current_task_start_time`和`get_current_task_syscall_times`。
6. 组装函数`sys_task_info`:调用`get_time_us`获取当前时间，与`get_current_task_start_time`结果的差值即`TaskInfo::time`; `syscall_times`可以通过调用`get_current_task_syscall_times`获得；`status`则可以直接设置为`TaskStatus::Running`。这一即可以获得`TaskInfo`的完整信息。

## 简答题

### 1.正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。 请同学们可以自行测试这些内容（运行 三个 bad 测例 (ch2b_bad_*.rs) ）， 描述程序出错行为，同时注意注明你使用的 sbi 及其版本。

- SBI版本：[rustsbi] RustSBI version 0.3.0-alpha.2, adapting to RISC-V SBI v1.0.0
`ch2b_bad_instructions.rs`是在U态调用了S态的汇编指令`sret`。
`ch2b_bad_register.rs`是尝试读取S态寄存器sstatus的值并放入一个变量中。
`ch2b_bad_address.rs`是尝试复写地址0x0的内容为0.

###2. 深入理解 trap.S 中两个函数 __alltraps 和 __restore 的作用，并回答如下问题:

1. L40：刚进入 `__restore` 时，a0 代表了什么值。请指出 __restore 的两种使用情景。

- a0寄存器代表函数的第一个输入参数。对于batch的情况，a0保存内核栈指针sp，所以__restrore开始时需要从a0中恢复内核栈指针sp。对于多道的情况，在__restore之前是执行__switch,而__switch的结尾已经恢复了下一任务的内核栈指针sp，所以__restrore不要再从a0中恢复内核栈指针。

2. L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。
```
ld t0, 32*8(sp)
ld t1, 33*8(sp)
ld t2, 2*8(sp)
csrw sstatus, t0
csrw sepc, t1
csrw sscratch, t2
```
- 这段代码首先从内核栈中加载了t0,t1,t2三个寄存器，这三个都是临时寄存器。然后用csrrw将t0中的值写入sstatus寄存器，由于sstatus的SPP字段保存CPU特权级，所以这一步是恢复用户态的特权级。将t1的值写入sepc寄存器，sepc保存Trap完成后进入用户态要执行的下一条指令。将t2的值写入sscratch寄存器，由于_alltraps时sscratch寄存器用于保存用户栈指针，所以这里在恢复用户栈指针。经过这几条指令：权限被修改；获得了下一条要执行的用户态指令；用户栈被恢复。这样进入用户态后可以继续执行。

3. L50-L56：为何跳过了 x2 和 x4？
```
ld x1, 1*8(sp)
ld x3, 3*8(sp)
.set n, 5
.rept 27
   LOAD_GP %n
   .set n, n+1
.endr
```
- x2即栈指针寄存器sp，__alltraps在这一位置存储的是临时寄存器t2的值。x4/tp是线程寄存器，在这里没有用到。

4. L60：该指令之后，sp 和 sscratch 中的值分别有什么意义？
```csrrw sp, sscratch, sp```
- 指令之后，sp是用户栈指针，sscratch是内核栈指针。

5. `__restore`：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？
- sret指令。这一指令会按照sstatus的SPP字段设置特权级，并跳转到sepc寄存器指向的指令执行。

6. L13：该指令之后，sp 和 sscratch 中的值分别有什么意义？
```csrrw sp, sscratch, sp```
- 指令之后，sp是内核栈指针，sscratch是用户栈指针。

7. 从 U 态进入 S 态是哪一条指令发生的？
- ```csrrw sp, sscratch, sp```，这条指令之后，切换到内核栈。

## 荣誉规则



1.在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

        《未与他人对实验进行交流》

2.此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

        《未参考其他资料》

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。