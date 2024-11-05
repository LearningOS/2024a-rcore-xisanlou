# 第六章报告

## 实验实现的功能

1. 实现sys_linkat/sys_unlinkat功能：
    （1）sys_linkat/sys_unlinkat中调用独立函数link_at/unlink_at进行文件链接和反链接操作。这两个函数是对全局结构对象ROOT_INODE的同名方法的包装。
    （2）由于ROOT_INODE是Inode结构的实例，所以新建Inode.link_at/Inode.unlink_at方法。
    （3）在DiskInode结构体中新加link_num成员记录链接数，并在DiskInode.initialize中将其初始化为1。通过Inode.increase_link_num/Inode.decrease_link_num方法进行升降，并在Inode.link_at/Inode.unlink_at方法内调用。
    （4）Inode.link_at/Inode.unlink_at会导致目录Inode中的目录项增加/减少，从而导致Inode长度的增减。原代码中已提供Inode.increase_size,我新实现了Inode.decrease_size。
    （5）unlink_at时，如果link_num降为零，则需调用Inode.clear清理Inode，然后用EasyFileSystem.dealloc_inode进行回收，这一方法也是新实现的。


2. 实现sys_linkat功能：
    （1）sys_fstat通过用户的TaskControlBlock.fd_table获得用于打开文件的句柄。这一句柄实际是OSInode对象，并通过File trait对外提供服务。这样需在File trait内新增一个get_fstat函数，它返回文件的Stat结构。为了让已使用File trait的代码可以继续使用，需在trait内为get_fstat提供默认实现。
    （2）在OSInode结构的impl File中，重新定义实现get_fstat, 它组装并返回文件的Stat结构。
    （3）在组装Stat结构时，获取其成员信息的方式是：Inode.get_id获得Inode的ID；Inode.get_type获得Inode的类型；Inode.get_link_num获得链接数。
    （4）Inode本身包含block_id和block_offset, Inode.get_id使用这两个值，并调用底层的EasyFileSystem.get_inode_id计算出Inode的ID号，这个方法也是新实现的。
    （5）编写Inode.get_type调用底层DiskInode.get_type获取类型。
    （6）编写Inode.get_link_num使用底层DiskInode.link_num获取链接数。

## 简答题

1. 在easy-fs中，root inode起着什么作用？如果root inode损坏了，会发生什么？
    答：在easy-fs中，root inode是唯一的目录Inode，所有文件都是位于root inode下。如果root inode损坏，则已创建的文件目的录项和文件名都会不可用，不能根据文件名查找和访问文件，但文件本身的内容不一定损坏。因为只有一个根目录，可以通过扫描data area的块，从中找到root inode对应的实际目录项所在的块，从而恢复root inode。












## 荣誉规则

1.在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

        《未与他人对实验进行交流》

2.此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

        《未参考其他资料》

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
