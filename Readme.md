rrtos
=====

This is a learning project to implement a Real-Time Operating System (RTOS) using Rust.

It runs on the Raspberry Pi Pico board, which has two Cortex-M0+ cores RP2040 MCU.

The following fundamental RTOS functionalities have been implemented:
* Task Switching Using Cortex’s `svc` Instruction
* Multitasking with Time-Sliced Scheduler
    + Tasks are forcibly switched even if they don't yield execution.
    + Tasks in a waiting state consume no execution time, enabling delay functionality.
    + When the idle task is executed, the MCU enters sleep mode, reducing power consumption.
    + Priority-based task scheduling is not implemented, yet.
* Device Drivers
    + The kernel initializes devices, and multiple tasks use them.
    + Device drivers are provided as libraries and executed with application-level privileges without calling system calls.
* Mutex for Exclusive Control
    + The Cortex-M0+ core does not have atomic instructions, and RP2040's dual-core design makes interrupt-based exclusive control unsafe. A Mutex using the spinlock mechanism provided by RP2040 is implemented.
* A global allocator is implemented. The `alloc` crate features like `Box` and `Vec` are available.


----

これは、RustでReal Time OS(RTOS)を実装する、学習のためのプロジェクトです。

Cortex-M0+コアを2つ搭載したRP2040, Raspberry Pi Picoのボード上で動作します。

RTOSの基本的な機能として、次のものが実装されています。

* Cortexの`svc`命令を利用したタスク切換え。
* 複数のタスクを作成して、スケジューラが時分割でタスクを実行。
    + タスクが協調的に実行権を譲らなくても強制的に切り替えます。
    + タスクが待機状態にあると実行時間を消費しません。これによりディレイが実現できます。
    + アイドルタスクが実行されるとMCUがスリープ状態になり消費電力を低減します。
    + 優先度つきタスクは実装されていません。
* デバイスドライバ。
    + カーネルがデバイスを初期化し、複数のタスクからデバイスにアクセスできます。
    + デバイスドライバはライブラリとして実行され、システムコールを介さず、アプリケーションの権限で実行されます。
* 排他制御のための`Mutex`
    + Cortex-M0+はアトミック命令がありません。また、RP2040はデュアルコアなので割り込み禁止での排他制御は安全ではありません。RP2040にそなわるスピンロックを利用した`Mutex`を実装しています。
* グローバルアロケータを実装して、`Box`や`Vec`などの`alloc`クレートが使えます。

---

* 本レポジトリからソースコードとヒストリが参照できます。
* [設計と実装の解説](https://nkon.github.io/Rust-Rtos/)
* [GDBメモ1](https://nkon.github.io/Gdb-basic/)
* [GDBメモ2](https://nkon.github.io/Gdb-Memo/)
* [Rustデータ構造メモ](https://nkon.github.io/CheatSheet/)
* [実装中のノート](NOTE.md)
* [DeepWiki.comによるドキュメント](https://deepwiki.com/nkon/rrtos)
