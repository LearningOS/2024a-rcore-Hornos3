# Completed Work

本实验完成了任务所要求的全部内容，在实现过程中，发现了ch4中对于内存映射实现的一些缺陷并恢复。另外，本实验对于调度算法的实现是基于`BTreeSet`完成，对`TaskControlBlock`实现了`Ord`Trait，按照`stride`从小到大排列。考虑到`BTreeSet`中不存在相等元素，另外添加了对`pid`的比较使得不存在相等的`TaskControlBlock`。这使得无需对所有任务进行遍历。在任务调度时取出`stride`值最小的任务，在添加对应的`pass`值之后重新添加回`BTreeSet`中，取出时自然会取出`stride`最小的任务。

# Problems
## Problem 01

stride 算法原理非常简单，但是有一个比较大的问题。例如两个 pass = 10 的进程，使用 8bit 无符号整形储存 stride， p1.stride = 255, p2.stride = 250，在 p2 执行一个时间片后，理论上下一次应该 p1 执行。

实际情况是轮到 p1 执行吗？为什么？

**My Answer**

---

因为会产生整数溢出，`p2`执行一个时间片之后它的`stride`会被截断为一个小整数，因此下一次依然会让`p2`执行。

## Problem 02

我们之前要求进程优先级 >= 2 其实就是为了解决这个问题。可以证明， 在不考虑溢出的情况下 , 在进程优先级全部 >= 2 的情况下，如果严格按照算法执行，那么 STRIDE_MAX – STRIDE_MIN <= BigStride / 2。

为什么？尝试简单说明（不要求严格证明）。

**My Answer**

---

反证法。假设在时间片t1结束之后，STRIDE_MAX (对应进程pid为p1) – STRIDE_MIN (对应进程pid为p2) <= BigStride / 2，那么t1时间片不可能执行p1，因为如果执行p1，由于优先级大于等于2，那么这个时间片执行之前 stride(p1) >= STRIDE_MAX - BigStride / 2 > STRIDE_MIN，因此其只能执行p3，其中t1结束时 stride(p3) <= STRIDE_MIN + BigStride / 2。

同理，在t1之前的t2时间片，p1同样不可能执行；在t2之前的t3时间片，p1同样不可能执行；......

倒推到整个系统执行的第1个时间片T1，从T1到t1的所有时间片p1都不能执行，因此T1之前stride(p1)=STRIDE_MAX，这显然矛盾，因为所有进程在一开始被加载时的stride值都为0，0≠stride(p1)。

故原命题成立。

## Problem 03

已知以上结论，考虑溢出的情况下，可以为 Stride 设计特别的比较器，让 BinaryHeap<Stride> 的 pop 方法能返回真正最小的 Stride。补全下列代码中的 partial_cmp 函数，假设两个 Stride 永远不会相等。

```rust
use core::cmp::Ordering;

struct Stride(u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let ss = &self.inner.exclusive_access().stride;
        let os = &other.inner.exclusive_access().stride;
        if (ss < 0 && os > 0) { return Some(Ordering::Less) }
        else if (ss > 0 && os < 0) { return Some(Ordering::Greater) }
        match i8::partial_cmp(ss, os) {
            Some(Ordering::Equal) | None => panic!(),
            Some(x) => Some(x)
        }
    }
}

impl PartialEq for Stride {
fn eq(&self, other: &Self) -> bool {
    false
}
}
```

TIPS: 使用 8 bits 存储 stride, BigStride = 255, 则: (125 < 255) == false, (129 < 255) == true.

**My Answer**

---
如上所示，不过需要注意当所有进程的stride均变成负数时需要将stride全部重置为0。

# 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：
   无

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：
   无

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。