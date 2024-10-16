# Completed Work

在ch3中，我实现了要求的所有功能，即引入一个新的系统调用用来获取当前Task的信息，包括系统调用的次数、目前已经执行的秒数等。个人认为系统调用次数获取使用桶不方便，原因是需要复制的字节数量太多，目前不清楚下一个实验是否会继承上一个实验自己实现的内容，因此这里没有修改，更好的方案是使用bitmask与计数器结合的方式，可以省去很多对于0字节的复制。

# Problems
## Problem 01
正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。 请同学们可以自行测试这些内容（运行 三个 bad 测例 (ch2b_bad_*.rs) ）， 描述程序出错行为，同时注意注明你使用的 sbi 及其版本。

**My answer**

---

rustSBI使用的是本次训练营仓库的默认版本：`RustSBI-QEMU Version 0.2.0-alpha.2`。

### ch2b_bad_address.rs

这个程序尝试在0x0地址空间进行写入操作，运行后内核输出报错信息并将其杀死，因此后面的`panic!`无法被执行到。

内核输出内容：

```text
[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.
```

能够执行这段代码是因为在`os/src/trap/mod.rs`中定义了异常处理函数（`trap_handler`, line 48）

### ch2b_bad_instructions.rs

这个程序尝试在U态执行S态指令`sret`，同样会触发异常处理函数。该指令是用于从S态返回原来陷入S态的CPU态，因此在U态不可用。

内核输出内容：

```text
[kernel] IllegalInstruction in application, kernel killed it.
```

### ch2b_bad_register.rs

这个程序尝试运行不合法的指令，`csrr <reg1>, <reg2>`中的`reg1`应为通用寄存器，但这里却尝试使用`CSR`，因此与`ch2b_bad_instructions.rs`一样触发异常处理函数。

内核输出内容：

```text
[kernel] IllegalInstruction in application, kernel killed it.
```

## Problem 2
深入理解 trap.S 中两个函数 __alltraps 和 __restore 的作用，并回答如下问题:

### Problem 2.1
L40：刚进入 __restore 时，a0 代表了什么值。请指出 __restore 的两种使用情景。

**My Answer**

---

可以看到`os/src/task/context.rs`中：

```rust
// line 25
pub fn goto_restore(kstack_ptr: usize) -> Self {
    extern "C" {
        fn __restore();
    }
    Self {
        ra: __restore as usize,
        sp: kstack_ptr,
        s: [0; 12],
    }
}
```

这里将`kstack_ptr`作为参数传递。根据riscv64参数传递规则，`a0`应作为保存函数/方法的第一个参数的寄存器。因为这个函数的第一个寄存器类型为`usize`，因此只需要通过`a0`一个寄存器保存即可（对于`&str`这样的切片类型，Rust需要使用两个寄存器传递或返回，若作为返回值，Ghidra将这种函数调用的寄存器使用规范定义为[`__rustcall`](https://hornos3.github.io/categories/%E5%AD%A6%E4%B9%A0%E7%AC%94%E8%AE%B0/Rust%E9%80%86%E5%90%91%E7%B3%BB%E5%88%97/)）。故 **`a0`代表了内核的栈顶地址** 。

### Problem 2.2
L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。
```asm
// os/src/trap/trap.S, line 43
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    ld t2, 2*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    csrw sscratch, t2
```

**My Answer**

---

这里是从内核栈的指定位置取出保存的CSR，包括`sstatus`、`sepc`、`sscratch`。

根据[指引文档](https://learningos.cn/rCore-Camp-Guide-2024A/chapter2/4trap-handling.html)描述，这些寄存器对于返回用户态的作用是：
- `sstatus`：记录当前CPU状态，可在进入用户态后将CPU状态标记为U态。
- `sepc`：记录用户态最后执行的地址，可以通过该寄存器的值返回到正确的用户态地址。
- `sscratch`：指向用户栈空间顶部地址，可以通过该寄存器的值正确引导用户态`sp`到用户栈顶。

### Problem 2.3
L50-L56：为何跳过了 x2 和 x4？
```asm
// os/src/trap/trap.S, line 49
    # restore general-purpuse registers except sp/tp
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
```

**My Answer**

---

因为`x2`实际上是`sp`，`x4`实际上为`tp`，在当前内核栈中的内容还没有完全恢复时，不能修改这两个寄存器的值，否则栈顶指针变化后将无法恢复这些内容，因此需要到其他内容恢复完成之后再去恢复`x2`和`x4`。

### Problem 2.4
L60：该指令之后，sp 和 sscratch 中的值分别有什么意义？
```asm
// os/src/trap/trap.S, line 60
    csrrw sp, sscratch, sp
```

**My Answer**

---

这条指令交换了`sscratch`和`sp`的值，使得`sp`的值变成用户栈栈顶地址，`sscratch`变成Trap之前内核栈栈顶地址。

### Problem 2.5
`__restore`：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？

**My Answer**

---

状态切换发生在第61行的`sret`中。该指令会去读取`sstatus`寄存器，并根据其中的SPP设置CPU状态。随后程序返回到`sepc`寄存器中保存的地址开始执行。由于之前已经完成了用户态的其他状态恢复，因此`sret`后将继续U态程序的正常执行。

### Problem 2.6
L13：该指令之后，sp 和 sscratch 中的值分别有什么意义？
```asm
csrrw sp, sscratch, sp
```

**My Answer**

---

与Problem 2.4重复，详见2.4

### Problem 2.7
从 U 态进入 S 态是哪一条指令发生的？

**My Answer**

---
与Problem 2.5重复，详见2.5

# 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

无

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

无

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。