# Brainfuck JIT 虚拟机教程

当我们谈到 JIT 时，通常会想到 V8、JVM 之类的庞然大物，然后望而生畏，觉得 JIT 是一种极其高深复杂的技术。

但 JIT 也可以变得非常简单，我们不需要做完善的优化和分析，只要输入源码，输出机器指令，再执行，这和普通的文本处理程序没什么区别。

在本教程中，我们将用 Rust 语言实现一个简单的 Brainfuck JIT 虚拟机，逐步理解 JIT 技术。

教程地址：<https://nugine.github.io/bfjit>