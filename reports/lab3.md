# 第五章报告

## 实验实现的功能

1. 修复前几章实现的4个系统调用：sys_get_time,sys_task_info,sys_mmap,sys_munmap。造成失效的主要原因是：（1）任务控制块的状态参数现在被移动到TaskControlBlockInner结构中；（2）当前任务的TaskControlBlock以前是由TaskManager管理，现在被移动到Processor进行管理。这样相应的操作函数和方法都需要进行修改。

2. 实现sys_spawn系统调用。主要是在TaskControlBlock结构中实现spawn方法，参考fork/exec的实现，以fork方法的实现为骨架，但不复制用户的MemorySet，而是参考exec新建一个。再结合sys_fork和sys_exec函数的实现，进行修改，得到sys_spawn函数的代码。

3. 实现Stride调度算法。首先，需要在TaskControlBlockInner结构中添加stride和pass分别记录stride调度算法总的“长度”和每次增长的“步长”。这里主要的修改是TaskManager结构。以前是使用VecDeque先入先出的调度任务，现在改为使用BinaryHeap<Reverse<Arc<TaskControlBlock>>>每次弹出stride“最小”的任务。由于BinaryHeap的要求，需要为TaskControlBlock trait。另外，为了处理stride溢出的情况，参考rustlings中的一个算法题的实现，我在TaskManager中使用了双BinaryHeap队列的方式处理溢出的情况，在首次溢出发生时在add方法内切换push队列，pop时一个队列空则在fetch方法中切换到另一pop队列。

4. 实现sys_set_priority系统调用。首先，为TaskControlBlockInner实现一个set_pass方法，设置pass值。在（os/src/task/processor.rs）中将其包装为一个公开函数current_user_set_pass。在sys_set_priority函数内依次做3件事：（1）检查输入优先级_prio不能小于2；（2）计算pass值；（3）调用current_user_set_pass为当前用户设置pass值。

## 简答题

stride 算法深入

stride 算法原理非常简单，但是有一个比较大的问题。例如两个 pass = 10 的进程，使用 8bit 无符号整形储存 stride， p1.stride = 255, p2.stride = 250，在 p2 执行一个时间片后，理论上下一次应该 p1 执行。

1. 实际情况是轮到 p1 执行吗？为什么？
    p1不会执行。p2.stride = 250 + 10 = 260,发生了溢出。实际结果是：
    p2.stride = 250 + 10 - 255 = 5 仍然小于p1.stride = 255。所以下次仍然调度p2。

        我们之前要求进程优先级 >= 2 其实就是为了解决这个问题。可以证明， 在不考虑溢出的情况下 , 在进程优先级全部 >= 2 的情况下，如果严格按照算法执行，那么 STRIDE_MAX – STRIDE_MIN <= BigStride / 2。

2. 为什么？尝试简单说明（不要求严格证明）。
    因为进程优先级 >= 2，所以pass <= BigStride / 2。因为算法的步长增长方式是取出STRIDE_MIN的任务，其增长后的stride的可能最大值是“STRIDE_MIN + BigStride / 2”，此前调度的任务的stride值也不肯能大于它， 即 STRIDE_MAX <= STRIDE_MIN + BigStride / 2

        已知以上结论，考虑溢出的情况下，可以为 Stride 设计特别的比较器，让 BinaryHeap<Stride> 的 pop 方法能返回真正最小的 Stride。

3. 补全下列代码中的 partial_cmp 函数，假设两个 Stride 永远不会相等。

```rust
use core::cmp::Ordering;

struct Stride(u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.0.wrapping_sub(other.0) > BigStride / 2 {
            // stride overflow
            if self.0 < other.0 {
                Some(Ordering::Greater)
            } else {
                Some(Ordering::Less)
            }
        } else {
            Some(self.cmp(other))
        }
    }
}

impl PartialEq for Stride {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}
```

## 荣誉规则

1.在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

        《未与他人对实验进行交流》

2.此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

        《未参考其他资料》

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
