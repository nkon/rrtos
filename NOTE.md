# プロジェクトを作成する

```
❯ git clone https://github.com/rp-rs/rp2040-project-template rrtos
```

* プロジェクト名を "rp2040-project-template" -> "rrtos" に変更する。
* 不必要なファイルを削る。

```
~/s/r/rrtos on  main [!?] is 📦 v0.1.0 via 🦀 v1.82.0 took 5s 
❯ cargo run                                                       
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.03s
     Running `probe-rs run --chip RP2040 --protocol swd target/thumbv6m-none-eabi/debug/rp2040-project-template`
     Erasing sectors ✔ [00:00:00] [#########] 12.00 KiB/12.00 KiB @ 54.27 KiB/s (eta 0s )
 Programming pages   ✔ [00:00:00] [#########] 12.00 KiB/12.00 KiB @ 27.87 KiB/s (eta 0s )    Finished in 0.683s
INFO  Program start
└─ rp2040_project_template::__cortex_m_rt_main @ src/main.rs:27  
INFO  on!
└─ rp2040_project_template::__cortex_m_rt_main @ src/main.rs:68  
INFO  off!
└─ rp2040_project_template::__cortex_m_rt_main @ src/main.rs:71  
INFO  on!
```
* `cargo run` で動作することを確認する。
* VS Code のデバッガでステップ実行ができることを確認する。

# BSPの依存をなくす

* `Cargo.toml`
    + `rp-pico = "0.9"`を削除
    + 次を有効化
```
rp2040-hal = { version="0.10", features=["rt", "critical-section-impl"] }
rp2040-boot2 = "0.3"
```
* `src/main.rs`
    + `bsp::entry;` -> `cortex_m_rt::entry;`
    + `use rp_pico as bsp;`を削除
    + 次を追加
```
#[link_section = ".boot2"]
#[used]
pub static BOOT_LOADER: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;
```
    + `use bsp::hal::` -> `rp2040_hal::`
    + `gpio::Pins`を追加
    + `bsp::Pins::new` -> `Pins::new`
    + `let mut led_pin = pins.led.into_push_pull_output();` -> `let mut led_pin = pins.gpio25.into_push_pull_output();`
  
動作を確認する。


# Create SysTick handler

[https://docs.rust-embedded.org/book/start/exceptions.html]

* 初期設定
```rust
let mut syst = core.SYST;                              // SYSTモジュールを得る
                                                       // core.SYSTがdelayと競合しているのでそちらをコメントアウト
syst.set_clock_source(SystClkSource::Core);            // クロック源にコアクロックを設定
syst.set_reload(clocks.system_clock.freq().to_kHz());  // リロード値を設定。1kHzで割り込みがかかるように
syst.clear_current();
syst.enable_counter();
syst.enable_interrupt();
```

* ハンドラ
```rust
#[exception]                   // SysTickの場合は割り込み(interrupt)ではなく例外(exception)ハンドラ
fn SysTick() {
    static mut COUNT: u32 = 0; // 例外ハンドラ中では非再入が保証されているのでsafeでもstatic mutが使える
    *COUNT += 1;
    if *COUNT == 1000 {
        info!("SysTick");
        *COUNT = 0;
    }
}
```

# Memory Map

| start       | size  | description               |
| ----------- | ----- | ------------------------- |
| 0x1000_0000 | 0x100 | .boot2                    |
| 0x1000_0100 | 0xc0  | .vector_table             |
| 0x1000_01c0 |       | .text                     |
| 0x1000_2646 |       | .rodata                   |
| 0x1000_2dec |       | empty                     |
| 0x1020_0000 |       | end of FLASH              |
|             |       |                           |
| 0x2000_0000 |       | HEAP VVV                  |
|             |       |                           |
| 0x2003_f7b8 |       | STACK ^^^ (__stack_start) |
| 0x2003_f7b8 |       | .data                     |
| 0x2003_f7f0 |       | .bss                      |
| 0x2003_f800 | 0x400 | .uninit (APP_STACK)       |
| 0x2003_fc00 | 0x400 | .defmt_buffer             |
| 0x2004_0000 |       | end of SRAM               |
|             |       |                           |
| 0xE000_ED08 |       | VTOR                      |

* boot2の中で、VTORが0x1000_0100(=vector_table)を指すように、MSPが`vector_table[0]`\(=RESET_VECTOR\)の中身(=`__stack_start`=0x2003_f7b8)を指すようにセットされて、`.text`の先頭(=0x1000_0100)にジャンプする。詳細は[boot-k]()の解説を参照。
* フレームワークのリンカファイル(memory.x)が提供している`.uninit`セクションの中に、`APP_STACK`を割り当て、アプリケーション用のスタックとする。アプリケーションを実行するときは、スタックポインタを切り替えて、ここの領域をスタックとして使うようにする。


# コンテキストの切り替え

* [Cortex-M0+ Technical Reference Manual](https://developer.arm.com/documentation/ddi0484/c/?lang=en)
* [Armv6-M Architecture Reference Manual](https://developer.arm.com/documentation/ddi0419/e/?lang=en)


# スタックフレームの初期化

[cortex-m-rt](https://docs.rs/cortex-m-rt/latest/cortex_m_rt/)が提供する`ExceptionFrame`がスタックフレームを表す。

`core::mem::MaybeUninit`を使って未初期化領域を割り当てる。`cortex-m-rt`が提供するリンカファイル(link.x)では`ALIGN(4)`となっている。しかし、スタックフレームは8バイトアラインにする必要があるので、構造体(`AlignedStack`)を被せて`#[repr(align(8))]`の属性を修飾しておく。

セクション`.uninit`はcortex-m-rtが提供する`link.x`で定義されている。

SRAM領域(0x2000_0000..0x2004_0000)の中で、末尾の0x2003_fbc0..0x2004_0000はdefmtが使う。その上の領域に`APP_STACK`が割り当てられる。

## リセットハンドラ

[cortex-m-rt](https://docs.rust-embedded.org/cortex-m-rt/0.6.1/cortex_m_rt/index.html)を使っている場合、リセットハンドラは'cortex-m-rt'内で定義されている。そのリセットハンドラが呼ぶフックとして`#[pre_reset]`属性を付けた関数を定義しておくと、リセットハンドラが呼ばれたときに実行される。

```rust
const STACK_SIZE: usize = 1024;
const NTHREADS: usize = 4;

#[repr(align(8))]
struct AlignedStack(MaybeUninit<[[u8; STACK_SIZE]; NTHREADS]>);

#[link_section = ".uninit.STACKS"]
static mut APP_STACK: AlignedStack = AlignedStack(MaybeUninit::uninit());

#[pre_init]
unsafe fn pre_init() {
    let ptr = APP_STACK.0.as_ptr() as usize;
    let exception_frame: &mut ExceptionFrame = &mut *(ptr as *mut ExceptionFrame);

    exception_frame.set_r0(0);
    exception_frame.set_r1(0);
    exception_frame.set_r2(0);
    exception_frame.set_r3(0);
    exception_frame.set_r12(9);
    exception_frame.set_lr(0);
    exception_frame.set_pc(app_main as u32);
    exception_frame.set_xpsr(0x0100_0000);
}

fn app_main() -> ! {
    info!("app_main()");
    unsafe {
        asm!("svc 0");
    }
    loop {}
}

fn execute_process(addr: u32) {
    unsafe {
        asm!(
            "push {{r4,r5,r6,r7,lr}}",
            "msr psp, {addr}",
            "svc 0",
            "pop {{r4,r5,r6,r7,pc}}",
            addr = in(reg) addr,
        );
    }
}
```


# 特権モードの整理

Table B1-1より

* Mode: Handler, Thread。例外ハンドリングはハンドラモードで、それ以外はThreadモードで実行される。リセット直後はスレッドモード。
* Privilege: Privileged, Unprivileged。特権モードでは全ての命令が実行可能。非特権モードでは命令が制限される。CONTROL.nPRIVビットが1のときは非特権モード。Unprivileged拡張の実装はMPUによって任意。RP2040は実装あり。
* Stack pointer: Main(MSP), Process(PSP)。ハンドラモードでは常にMSPが使われる。CONTROL.SPSELが1のときはPSPが使われる。

| Mode    | Privilege    | Stack Pointer | Usage                                |
| ------- | ------------ | ------------- | ------------------------------------ |
| Handler | Privileged   | Main          | 例外ハンドラ                         |
| Thread  | Privileged   | Main          | OSの実行                             |
| Thread  | Privileged   | Process       | Unprivilegedがない場合のアプリの実行 |
| Thread  | Unprivileged | Main          | ???                                  |
| Thread  | Unprivileged | Process       | アプリの実行                         |

* 例外: Reset, NMI, HardFault, SVCall, Interrupts。コアでサポートしている割り込みは PendSV, SysTick。
* Handlerモードに移行するためには`SVC`命令を実行して`SVCall`ハンドラに入るか、SysTickなどの割り込みによってハンドラに入る(A4.9)。
* 特権モードでなければ実行できない命令=CPS, MRS, MSRが使える。
    * `CSPIE i`(Interrupt Enable, Set PRIMASK.PM to 0), `CSPID`(Interrupt Disable, Set PRIMASK.PM to 1)は、非特権モードの場合は無視される。
    * `MRS`(Read Special Register)。非特権モードでASPR、CONTROL「以外」を読み出すと0が帰る。
    * `MSR`(Write Special Register)。
* SP(r13)で有効な方のスタックポインタが参照される。 

# 例外からの復帰

`EXC_RETURN`という特別なアドレスにリターンする(`bx`または`PC`に`pop`する)。

* 0xFFFF_FFF1: ハンドラモードに戻る。レジスタ類はMSPから復帰され、その後の実行もMSPが使われる。
* 0xFFFF_FFF9: スレッドモードに戻る。レジスタ類はMSPから復帰され、その後の実行もMSPが使われる。(カーネル実行)
* 0xFFFF_FFFD: スレッドモードに戻る。レジスタ類はPSPから復帰され、その後の実行もPSPが使われる。(アプリ実行)


## SVCall ハンドラの実装


```rust
#[exception]
fn SVCall() {
    // info!("SVCall: lr={:x}", cortex_m::register::lr::read());
    unsafe {
        asm!(
            "pop {{r6, r7}}", // Adjust SP from function prelude "push {r7, lr};add r7, sp, #0x0"
            "ldr r4, =0xfffffff9", //If lr(link register) == 0xfffffff9 -> called from kernel
            "cmp lr, r4",
            "bne 1f",
            "movs r0, #0x3",
            "msr CONTROL, r0",     //CONTROL.nPRIV <= 1; set unprivileged
            "isb",                 // Instruction Synchronization Barrier
            "ldr r4, =0xfffffffd", // Return to Thread+PSP
            "mov lr, r4",
            "bx lr",
            "1:",
            "movs r0, #0",
            "msr CONTROL, r0", //CONTROL.nPRIV <= 0; set privileged
            "isb",
            "ldr r4, =0xfffffff9", // Return to Thread+MSP
            "mov lr, r4",
            "bx lr",
            options(noreturn),
        );
    };
}
```
* `svc`(Super Visor Call)命令を呼び出すと、`SVCall`例外が発生し、ハンドラに飛んでくる。
    + ハンドラに飛んでくる前に、`r0`,`r1`,`r2`,`r3`,`r12`,`lr`,`pc`,`xPSR`を、そのときのスタック上に積む。`exception_frame`と呼ぶ。
    + 通常、`sp`は4バイトアラインだが、`exception_frame`は8バイトアライン。必要に応じて、アライメントが挿入される。
* `#[excepton]`はcrotex-m-rtでの例外ハンドラサポート。
    + **ただし、コンパイラによって、先頭に`push {r7, lr};add r7, sp, #0x0`、末尾に`pop {r7, pc}`が自動で挿入される。`SVCall`の場合、これでスタックポインタがズレるので、最初に補正しておく**。
    + 本来ならば、アセンブラで書いてリンクするべき。
    + [`#[naked]`](https://docs.rs/naked-function/latest/naked_function/attr.naked.html)が使える可能性もあるが、`#[naked]`は純アセンブラ関数であることが必要。`#[exeption]`はRustマクロを利用しているので相反する。
    + ちなみに、`execute_process`も同様の問題(関数に入るときに`push {r7 lr}`して、出るときに`pop {r7 pc}`する)があるが、途中で`pop pc`して実行の流れを上書きしているので問題とならない。こちらは、引数をとっているので、`#[naked]`にできない。
* Thread+MSPから呼ばれた場合 `lr`が0xffff_fff9に、Thread+PSPから呼ばれた場合0xffff_fffdになっている。それを判別して、呼び出し元とは違うスタックポインタに戻るというコード。 

参考書「Rustで始める自作組み込みOS入門」はWioTerminalを使っている。MPUはATSAMD51P19、アーキテクチャはCortex-M4F(ARMv7-M)で、Thumb2 ISA(thumbv7em)が使える。

本記事のターゲットボードはRasPico RP2040で、アーキテクチャはCortex-M0+(ARMv6-M)、Thuumb ISA(thumbv6m)しか使えない。アセンブラの記述やMPUの制御が異なるので注意が必要。

ARMの呼び出し規約(AAPCS)では、r0とr1は返り値、r0-r3は引数、r4-r11は変数、r12はスクラッチレジスタ。r13はスタックポインタ(SP)、r14はリンクレジスタ(LR)、r15はプログラムカウンタ(PC)となっている。また、thumbv6mではldrのターゲットレジスタはr0-r7まで(3ビット)。このようにいったんリテラルをロードするような用途にはr4が適切。

# プロセス切り替え

## `SCVall`

### r4-r7を壊してはダメ

タスク切換えで`r0`,`r1`,`r2`,`r3`,`r12`は例外フレームから戻されるので自由に破壊して良い。それ以外のレジスタは呼び出し元で使用している可能性があるので破壊してはならない。

* `cortex-m-rt`の`#[exception]`を使うと、関数プロローグで`push {r7, lr}`, `add r7, sp, #0x0`される。これを補正するのに、先頭で`pop {r7}`(`r7`の復元) `pop {r2}`(pushされた`lr`をダミーpop)する。
* 即値ロードするためのレジスタは `r3`を使う。
    + 壊して良いのは`r0-r3`,`r12`。`r12`はスクラッチレジスタとして使われる。しかし、`push`/`pop`できるのは`r0-r7`+`lr`。


## execute_process

* インライン展開されるとレジスタ番号が変わってしまうので、`#[inline(never)]`をつけて、必ず関数として呼ばれる(第一引数が`r0`、第二引数が`r1`)ようにする。
* 関数プロローグで`push {r7, lr}`, `add r7, sp, #0x0`される。残りの`{r4,r5,r6}`を自力で`push`する。
* `limia`でr4-r7を`r1`が指すバッファに保存する。thumbv6(Cortex-M0+)の場合はr1が破壊されるので、事前に`push`しておく。
* `svc 0`をコールすると、`SVCall`経由でアプリケーションに切り変わる。アプリケーション側でさらに`svc 0`が呼ばれるので、`SVCall`経由で戻って来る。
* `r1`を`pop`して、`stmia`を使って`r4-r7`をバッファに保存する。
* 関数エピローグで`r7,pc`がpopされるので、`r4-r6`だけ自力で`pop`する。
* アプリケーション実行時に`psp`が変わっているので、値を返して、呼び出し側のアプリケーションスタックに保存しておく。


# ARM Thumb V6(Cortex-M0+) ABI

[Cortex-M0+ Technical Reference Manual](https://developer.arm.com/documentation/ddi0484/c)

メモリマップ(Cortex-M共通)
* 0x0000_0000..: コードメモリ
    + 0x0000_0000..: 内蔵ROM(ブート用)
    + 0x1000_0000..: 内蔵Flash(ユーザ用: RP2040の場合)
        - 0x1000_0000..0x1000_01c0: .boot2(RP2040の場合)
        - 0x1000_01c0..0x1000_0200: 割り込みベクタ(RP2040の場合)
        - 0x1000_0200..: ユーザコード(RP2040の場合)
* 0x2000_0000..: 内蔵RAM
* 0x4000_0000..: オンチップ・ペリフェラル
* 0x6000_0000..: 外部RAM
* 0xA000_0000..: 外部デバイス
* 0xE000_0000..: システムコンポーネント
  
[レジスタ](https://developer.arm.com/documentation/ddi0484/c/Programmers-Model/Processor-core-registers-summary)は`r0-r7`,`r8-r11`,`r12`,`r13(sp)`,`r14(lr)`,`r15(pc)`がある。さらに`PSR`、`PRIMASK`,`CONTROL`がある。

* Cortex-M0+では`r8-11`は使えない命令が多い。**今回はとりあえず無視する**
        - レジスタを指定するビット幅が3ビットのものがほとんど。4ビットのものもある。 

関数呼び出しのとき、引数が4つまでなら`r0-r3`を使ってわたす。それ以上ならスタックに積んでわたす。

* 関数の引数は4つまで(`self`を含む)が高速化につながる。
* 関数呼び出し=`r0-r3`にパラメータをセットして`bl`命令。`bl`命令は32ビット命令で、飛び先は`pc`相対で*/-4MB(+0x3f_ffff..-0x3f_ffff):`bl`命令の場合、または*/-16MB(+0xff_ffff..-0xff_ffff):`bl.w`命令の場合。遠すぎる場合は`blx <Rd>`命令が使われる。
    + 飛び先の関数は2byteアライメント。末尾のビットが`1`なら、Little Endianで飛び先の関数を実行する(ほとんどすべてコレ) 
+ `br`命令は、呼び出し時にリターンアドレスを`lr`にセットする。
    + Rustのコンパイラは先頭で`lr`を`push`し(プロローグ)、最後に`pop pc`する(エピローグ)ことによって、関数終了時に呼ばれたときの`lr`にジャンプする(`pc`にその値がセットされる)。  
* 戻り値は`r0`,`r1`を使って返す。
* 呼ばれた側では`r4-r7`,`r8-r11`は壊してはならない。
    + 必要に応じて`push`しておく。
* Rustのコンパイラは、
    + 関数の入口で`push {r7, lr}`,`add r7, sp, #0x0`して(プロローグ)、
        - 必要に応じて(関数中で使っていれば)、さらにレジスタをスタックに積む
    + 出口で`pop {r7, pc}`する(エピローグ)。`pc`が書き換わった時点で関数リターン。
    + その後ろにはオーバーラン用のトラップコードが存在する。

例外が発生したときは`r0-r3`,`r12`,`lr`,`pc`,`xPSR`がスタックに積まれる(exception frame)。Exception Frameは2ワード(8️バイト)アライン。必要に応じてアライメントが調整される。
* 例外から復帰するときは、復帰先のスタック(`msp`または`psp`)からException frameの内容が`pop`される。
 
動作モード

* 起動時は Thread mode、MSP, Privileged
* 例外ハンドラ、割り込みハンドラでは Handler mode, MSP, Privileged
* OS(を使うとき)は Thread mode, MSP, Privileged
* アプリケーションは Thread mode, PSP, Unprivileged
* アプリケーションモードからカーネルモードに移行するためには
    + 例外の発生(SysTickハンドラなど)
    + `svc`命令の実行`SVCall`ハンドラが実行される
        - `SVCall`ハンドラは短時間で処理を終え、残りは低優先度の`PendSV`ハンドラが行う
* ハンドラモードからリターンするとき、次のアドレスにリターン(`bx`)すると処理が分岐する(`EXC_RETURN`)。同様に例外ハンドラが呼ばれたときの`lr`は呼び出し元のコンテキストが入る
    + `0xFFFFFFF1`: Return to Handler Mode. Exception return gets state from the Main stack. On return execution uses the Main Stack.
    + `0xFFFFFFF9`: Return to Thread Mode. Exception return gets state from the Main stack. On return execution uses the Main Stack.
    + `0xFFFFFFFD`: Return to Thread Mode. Exception return gets state from the Process stack. On return execution uses the Process Stack.


# Mutexの実装

割り込みなどがあると、どうしてもグローバル変数が必要になる。
Rustではグローバル変数へのアクセスは、マルチスレッド環境で競合状態を生むため、厳しく制限されている。

競合状態を回避するためには、手動でロックを実装しなければならない。
代表的にはMutexを使う。

* `no_std`環境では`spin`クレートを使うことが一般的。
    + `spin`はスピンロック(ロックを取ろうとして、取れないときは無限ループで取れるまで待つ)を提供する。スピンロックの実装に`AtomicBool::compare_exchange()`を用いており、Cortex-M3(ARMv7-M:thumbv7m)では`LDREX`,`STREX`命令を使って実装されている。しかしCortex-M0+(ARMv6-M:thumbv6m)ではその命令がないので、コンパイルエラーとなる。
    + `lazy_static`クレートも内部で`spin`を使っているので、Cortex-M0+では同様に使えない。
* `cortex-m`クレートが独自に`cortex_m::interrupt::Mutex`を提供しているが、実装には「割り込み禁止」を使っている。Cortex-Mの場合は割り込み禁止はコアごとに設定されるので、これはシングルコアでないと正しく動作しない。RP2040はマルチコアなので`cortex_m::interrupt::Mutex`は使えない。
* RP2040はハードウエアのスピンロックをサポートしている。`rp2040::sio::SpinLock`だ。ハードウエアとして32個の1ビットレジスタがあり、core0からもcore1からもロックとして動作する。`struct`としては、`rp2040::sio::SpinLock0`..=`rp2040::sio::SpinLock31`まで、個別に提供されている。次の4つのメソッドが生えている。`&self`を引数に取るわけではなく、それぞれのSpinLockレジスタ(構造体)のクラスメソッドである。
    + `pub fn try_claim() -> Option<Self>`Try to claim the spinlock. Will return Some(Self) if the lock is obtained, and None if the lock is already in use somewhere else.
    + `pub fn claim() -> Self`Claim the spinlock, will block the current thread until the lock is available.Note that calling this multiple times in a row will cause a deadlock
    + `pub fn claim_async() -> Result<Self, Infallible>`Try to claim the spinlock. Will return WouldBlock until the spinlock is available.
    + `pub unsafe fn release()`Clear a locked spin-lock. Safety: Only call this function if you hold the spin-lock.
    + `critical-section`の実装に`SpinLock31`が使われているので他では使えない。
* `rp2040_hal`クレートでは`features="critical-section-impl"`を有効にした場合、`SpinLock31`がライブラリ内部で`critical_section`の制御に使われる。
    + `critical_section_impl`モジュールは`pub`ではないので、`rp2040_hal`の外では使えない。

Rust+ベアメタル+RTOSの分野ではCortex-M3での例題が多く公開されているが、RP2040では、かなり差異が大きくなる。

今回は自前で、`rp2040::sio::SpinLock0`を使ってMutexを実装する。

RustのMutexは、Mutex変数をlockするのではなく、ジェネリック型として実装される。

* C言語などのようにロック変数(Mutex変数)を使う場合は、ロック変数をロックしてから、ロック変数をアンロックするまでがクリティカル区間となり、同じロック変数を共用している限り、他のスレッドはクリティカル区間に入ってこない。
* Rustの場合は、データにロックが付く。データをロックしてから、データをアンロックするで、他のスレッドでそのデータをロックできないので、データが保護される。通常、アンロックは`Drop`トレイトによって実施される。スコープを外れたときに、自動的に`Drop`トレイトによって自動でアンロックされる。
    + ロック変数とデータが一体化されているので、データにアクセスするときにロックをし忘れることがない。

```rust
pub struct Mutex<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}
```

これに、`new()`, `lock()`, `unlock()`が実装される。

* `new()`は引数を`UnsafeCell<T>`に格納する。
* `lock()`は内部のロックを取り、`MutexGuard<'_, T>`型を返す。
* `unlock()`は内部のロックを返す。
* 内部のロック変数(`locked`)は`AtmicBool`型だがアトミック操作(`compare_exchange()`)を用いていないので、`rp2040::sio::SpinLock`でガードする。
* ロック変数は複数スレッドからアクセスされる可能性があるので`Ordering`を指示する。これはメモリバリア命令などにコンパイルされ、それらはCortex-M0+にも存在する。

```rust
pub struct Mutex<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(value),
        }
    }
    pub fn lock(&self) -> MutexGuard<'_, T> {
        while self.locked.load(atomic::Ordering::Acquire) {
            // 他のスレッドがlockedを開放するまで待つ
            core::hint::spin_loop()
        }
        let _lock = Spinlock0::claim();
        self.locked.store(true, atomic::Ordering::Release);
        MutexGuard::new(self)
        // SpinLock0自体はここでドロップ=>releaseされる
    }
    fn unlock(&self) {
        if !self.locked.load(atomic::Ordering::Acquire) {
            return;
        }
        let _lock = Spinlock0::claim();
        self.locked.store(false, atomic::Ordering::Release);
    }
}
```

`lock()`が`MutexGuard`型を返すのがポイント。`MutexGuard`型の変数が有効ならばロックも有効、ということ。ユーザが直接`MutexGuard`を操作することは無いので、メソッドはすべてプライベート(`pub`ではない)。

`MutexGuard<'a, T>`という型をつくる。

```rust
pub struct MutexGuard<'a, T> {
    lock: &'a Mutex<T>,
}
```

参照(`Deref`、`DerefMut`)のトレイとを実装し、中身に透過的にアクセスできるようにする。

```rust
impl<'a, T> MutexGuard<'a, T> {
    fn new(lock: &'a Mutex<T>) -> Self {
        MutexGuard { lock }
    }
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}
```
そして、`Drop`のトレイトを実装し、スコープを外れたら`unlock()`するようにする。

```rust
impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}
```

最後に、`Mutex`,`MutexGuard`に`Sync`を実装して、複数スレッドからのアクセスを許可する。これは、プログラマがロックを使って責任をもって管理するから共有アクセスを可能にしてください、というコンパイラへのお願い。なので`unsafe`となる。

* 別のスレッドに送ることができる=`Send`
* 別のスレッドと共有できる=`Sync`

```rust
unsafe impl<T> Sync for Mutex<T> {}
unsafe impl<T> Sync for MutexGuard<'_, T> {}
```

こうやって作った`Mutex`を使えば、グローバルデータに安全(`unsafe`を使わずに)にアクセスできる。

```rust
struct Count(u32);

impl Count {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
    pub fn incr(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

// Mutex::new, Count::new が const fn なので、static変数を初期化できる
static SYSTICK_COUNT: Mutex<Count> = Mutex::new(Count::new(0));

pub fn systick_count_incr() {
    // lockを取って、UnsafeCell<>の中の値を操作する(mutでなくてもOK:内部可変性)
    SYSTICK_COUNT.lock().incr();
}

pub fn systick_count_get() -> u32 {
    SYSTICK_COUNT.lock().0
}
```

# alloc::boxed::Box, Box::leak(), GlobalAlloc

値をスタック上に割り当てるのではなくヒープ上に割り当てる場合、通常の`std`環境では`Box`を使う。`no_std`の`core`クレートでは提供されていないが、`no_std`環境でも、メモリアロケータを実装することで`alloc`クレートが使え、`Box`や`Vec`などが使える。

他の手段として`heapless`クレートもあるが、アトミック命令に依存しているのでCortex-M0+では使えない。

簡単にやるには、ヒープ領域を固定で割り当て、BumpPointerAllocatorという、使用済み領域を示すポインタが進むだけで、開放してもメモリが戻らないタイプのアロケータを実装する。

[https://tomoyuki-nakabayashi.github.io/embedded-rust-techniques/03-bare-metal/allocator.html]

こちらの例題のメモリアロケータはバグがあるので、本家のものを参照するのが良い。

[https://docs.rust-embedded.org/book/collections/]

この実装は簡易なもので、マルチコア環境でのデータ競合を想定していないので、実際に使うにはMutexなどで保護する必要がある。

`GlobalAlloc`を定義して`alloc::boxed::Box`を使えば、データをヒープ上に割り当てることができる。

ただし、そのヒープ上のデータはグローバル(`'static`ライフタイム)変数からは参照できない。なぜかというと、ヒープ上のデータは`'static`よりもライフタイムが短いから。`Box::leak()`を使えば、メモリリークを承知の上で、ヒープ上のデータを`'static'`な変数に参照させることができる。

マニュアルにも書かれているが、`Box::leak()`はメンバ関数ではなく、クラス関数なので次の例のように使う。

[https://doc.rust-lang.org/alloc/boxed/struct.Box.html#method.leak]

```rust
// グローバルなスケジューラを定義する。複数スレッドからアクセスされるのでMutexで囲う
static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());

//...

// `.uninit`領域に`APP_STACK`を確保する
    #[link_section = ".uninit.STACKS"]
    static mut APP_STACK: AlignedStack = AlignedStack(MaybeUninit::uninit());

    // ヒープ上に`Task`構造体を割り当てる  
    let task = Box::new(Task::new(
        unsafe { &mut *addr_of_mut!(APP_STACK) },
        app_main,
    ));

    // リストアイテム構造体をヒープ上に割り当て、leak()で`staticライフタイムを与える
    let item: &'static mut ListItem<Task> = Box::leak(Box::new(ListItem::new(*task)));

    // グローバルなスケジューラにタスク構造体を登録する
    SCHEDULER.lock().push_back(item);
```

# `#![cfg_attr(test, no_std)]`

`#![cfg_attr(A, B)]`で、もし`A`なら`B`をセットする、という意味。

linked_list.rsのユニットテストは次のように実行する。

```
❯ rustc --test src/linked_list.rs
```
こうすると、ネイティブの linked_list という実行ファイルができる

```
❯ ./linked_list 

running 1 test
test test::test_list ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```



# gdb memo

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


# links

* [Procedure Call Standard for Arm Architecture (AAPCS)](https://developer.arm.com/documentation/107656/0101/Getting-started-with-Armv8-M-based-systems/Procedure-Call-Standard-for-Arm-Architecture--AAPCS-)
* [ARM®v6-M Architecture Reference Manual](https://developer.arm.com/documentation/ddi0419/latest/)
* [Cortex-M0+ Technical Reference Manual](https://developer.arm.com/documentation/ddi0484/latest/)
* [ARM® Cortex®-M mbed™ SDK and HDK deep-dive](https://os.mbed.com/media/uploads/MACRUM/cortex-m_mbed_deep-dive_20140704a.pdf)
* [ARM Cortex-M RTOS Context Switching](https://interrupt.memfault.com/blog/cortex-m-rtos-context-switching)
* [FreeRTOS(Cortex-M)のコンテキストスイッチ周りを調べてみた](https://zenn.dev/lowlvengineer/articles/f87393345bb506)
* [rp2040のPendSVでコンテキストスイッチをしよう](https://qiita.com/DanfuncL/items/b8b5a8bd03973880acfd)
* [ARM関連(cortex-Mシリーズ)のCPUメモ](https://qiita.com/tom_S/items/52e4afdb379dff2cf18a)
* [ARM Cortex-M 32ビットマイコンでベアメタル "Safe" Rust](https://qiita.com/tatsuya6502/items/7d8aaf3792bdb5b66f93)
* [Rustで始める自作組込みOS入門](http://garasubo.com/embedded-book/) 基本的にこのページ(書籍化もされている)に沿ってやっていくが、ターゲットがRasPi Picoで、コアがCortex-M0+である。本書が使っているCortex-M4とは異なる部分があり、対応が必要。
* [Cortex-M0+ CPU Core and ARM Instruction Set Architecture](https://wordpress-courses1920.wolfware.ncsu.edu/ece-461-001-sprg-2020/wp-content/uploads/sites/106/2020/01/ARM-ISA-and-Cortex-M0.pdf)


