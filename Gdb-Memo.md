---
layout: post
title: GDB 使い方メモ
category: blog
tags: rust embedded RasPico gdb cortex-m
---

RTOSを作ったとき、アセンブラレベルでMCUの動作を追いかける必要があり、gdbが必須である。[以前に自分で書いたメモ](https://nkon.github.io/Gdb-basic/)も非常に役にたった。今回も、使い方メモをまとめておくと、何年かあとの自分の役に立つだろう。

## cargo objdump

ディスアセンブリを生成して、エディタで開いておくと見通しが良くなる。

```
❯ cargo objdump -v -- --disassemble-all > asm.S
```

## open-ocd

OpenOCD serverを起動する。

```
❯ openocd -f interface/cmsis-dap.cfg -f target/rp2040.cfg -c "adapter speed 5000"
```

## gdb

よく使うコマンド

* `monitor reset init`: ターゲットをリセットする
* `b main`: `main`にブレークポイントを設定
* `c(continue)`: 実行を継続してブレークポイントで停止
* `i b(info breakpoints)`: ブレークポイントの情報を表示
* `i r(info register)`: レジスタを表示
* `disp $pc`と設定しておくことで、コマンド実行後に毎回$pc(=次に実行する行)を表示する
* `disas`: ディスアセンブリ
* `b *0x10000270`: 関数名でうまくブレークポイントが設定できない場合はアドレスでブレークポイントを指定
* `x/32xw $msp`: `x`はメモリダンプのコマンド。`$msp`が指すアドレスから32ワードを16進表示する。この場合はスタックを表示している。

### 実行例

```
❯ arm-none-eabi-gdb target/thumbv6m-none-eabi/debug/rrtos

Reading symbols from target/thumbv6m-none-eabi/debug/rrtos...
(gdb) target remote localhost:3333                                       # リモートのOpenOCDに接続する
Remote debugging using localhost:3333
rrtos::__cortex_m_rt_main_trampoline () at src/main.rs:95
95      #[entry]
(gdb) monitor reset init                                                 # ターゲットをリセットする
[rp2040.core0] halted due to debug-request, current mode: Thread 
xPSR: 0xf1000000 pc: 0x000000ee msp: 0x20041f00
[rp2040.core1] halted due to debug-request, current mode: Thread 
xPSR: 0xf1000000 pc: 0x000000ee msp: 0x20041f00
(gdb) b main                                                             # まずはmainにブレークポイント
Breakpoint 1 at 0x100003fc: file src/main.rs, line 95.
Note: automatically using hardware breakpoints for read-only addresses.
(gdb) c                                                                  # 実行
Continuing.

Breakpoint 1, rrtos::__cortex_m_rt_main_trampoline () at src/main.rs:95
95      #[entry]
(gdb) disas                                                              # ディスアセンブリ
Dump of assembler code for function main:
   0x100003f8 <+0>:       push    {r7, lr}
   0x100003fa <+2>:       add   r7, sp, #0
=> 0x100003fc <+4>:       bl  0x10000400 <_ZN5rrtos18__cortex_m_rt_main17h8cc62a2f3875e4d5E>
End of assembler dump.
(gdb) i b                                                                 # info breakpoints
Num     Type           Disp Enb Address    What
1       breakpoint     keep y   0x100003fc in rrtos::__cortex_m_rt_main_trampoline at src/main.rs:95
        breakpoint already hit 1 time
(gdb) i r                                                                 # info registers
r0             0x2003f7f0          537130992
r1             0x2003f7f0          537130992
r2             0x10002d10          268446992
r3             0x74                116
r4             0x0                 0
r5             0x20041f01          537140993
r6             0x18000000          402653184
r7             0x2003f7b0          537130928
r8             0xffffffff          -1
r9             0xffffffff          -1
r10            0xffffffff          -1
r11            0xffffffff          -1
r12            0x4001801c          1073840156
sp             0x2003f7b0          0x2003f7b0
lr             0x100001e7          268435943
pc             0x100003fc          0x100003fc <rrtos::__cortex_m_rt_main_trampoline+4>
xPSR           0x61000000          1627389952
msp            0x2003f7b0          0x2003f7b0
psp            0xfffffffc          0xfffffffc
primask        0x0                 0
basepri        0x0                 0
faultmask      0x0                 0
control        0x0                 0
(gdb) disp $pc                                                           # disp $pc としておくと、コマンド実行後に$pc(=実行する行)を表示してくれる
1: $pc = (*mut fn ()) 0x100003fc <rrtos::__cortex_m_rt_main_trampoline+4>
(gdb) i disp                                                             # info disp で何がdisplayに登録されているかを確認できる
Auto-display expressions now in effect:
Num Enb Expression
1:   y  $pc
(gdb) b *0x10000270                                                      # 関数名でうまくブレイクポイントがセットできないなら、ディスアセンブリを見てアドレスで(*0x....)
Breakpoint 3 at 0x10000270: file src/main.rs, line 64.
(gdb) c
Continuing.

Breakpoint 3, rrtos::execute_process (sp=0) at src/main.rs:64
64      fn execute_process(sp: u32) {
1: $pc = (*mut fn ()) 0x10000270 <rrtos::execute_process>
(gdb) x/32xw $msp                                                         # x:メモリダンプ。$mspのアドレスから32ワード(1ワード=32bit)を16進表示  
0x2003f6dc:       0x00000002      0x2003f6f0      0x2003f7d0      0x2003f730
0x2003f6ec:       0x1000124d      0x2003f708      0x2003f728      0x2003fbe0
0x2003f6fc:       0x2003f6f0      0x2003f708      0x100010d7      0x2003fbe0
0x2003f70c:       0x2003f6f0      0x2003f7d0      0x00000000      0x2003f728
0x2003f71c:       0x10001237      0x02000002      0x2003fbe0      0x02000000
0x2003f72c:       0xd0000014      0x2003f7a8      0x100005af      0x3f851e6f
0x2003f73c:       0x00000000      0x00000000      0x00000000      0x10000209
0x2003f74c:       0x01000000      0x00000000      0x01000000   
```
