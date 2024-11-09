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
    + `bsp::Pins::new(` -> `Pins::new(`
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

# Create SVCall handler

* [Cortex-M0+ Technical Reference Manual](https://developer.arm.com/documentation/ddi0484/c/?lang=en)
* [Armv6-M Architecture Reference Manual](https://developer.arm.com/documentation/ddi0419/e/?lang=en)

参考書「Rustで始める自作組み込みOS入門」はWioTerminalを使っている。MPUはATSAMD51P19、アーキテクチャはCortex-M4F(ARMv7-M)で、Thumb2 ISA(thumbv7em)が使える。

本記事のターゲットボードはRasPico RP2040で、アーキテクチャはCortex-M0+(ARMv6-M)、Thuumb ISA(thumbv6m)しか使えない。アセンブラの記述やMPUの制御が異なるので注意が

```rust
#[exception]
fn SVCall() {
    unsafe {
        asm!(
            "ldr r1, =0xfffffff9", //If lr(link register) == 0xfffffff9 -> called from kernel
            "cmp lr, r1",
            "bne 1f",              // lr を 0xfffffff9 と比較して、違えば後方のラベル1にジャンプ
            "movs r0, #1",         // thumbv6m では mov命令はなく movs命令しかない
            "msr CONTROL, r0",     // CONTROL.nPRIV <= 1; set unprivileged
            "isb",                 // Instruction Synchronization Barrier
            "ldr r1, =0xfffffffd", // 0xffff_fffc + 0x01(call with Thumb inst.)
            "mov lr, r1",          // thumbv6m の mov は8ビットの即値しか取れない→いったんr1にldrする
            "bx lr",               // lrをカーネルスタックにセット
            "1:",                  // カーネルスタックから呼ばれた場合
            "movs r0, #0",
            "msr CONTROL, r0",     //CONTROL.nPRIV <= 0; set privileged
            "isb",
            "ldr r1, =0xfffffff9", // 0xffff_fff8 + 0x01(call with Thumb inst.
            "mov lr, r1",
            "bx lr",               // lrをアプリケーションスタックにセット
            options(noreturn),
        );
    };
}
```