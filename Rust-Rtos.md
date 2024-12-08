---
layout: post
title: RustでRTOSを作る(RP2040, Cortex-M0+)
category: blog
tags: rust embedded RasPico RTOS cortex-m
---

Raspberry Pi Pico(RP2040)上で動作するRTOSを自作する。技術コンセプトの実証のためのプロトタイプだが、タスク切換え、SysTickのハンドリング、タスクの停止、システムコール、などの機能を持っている。類似の多くの情報があるが、Cortex-M3のマイコンをターゲットにしたものが多い。RP2040はCortex-M0+コアであり、更にマルチコアであり、それらの既存の情報がうまく適用できない場面が多い。本記事は、とくに差異において注意する点をとりあげた。また、組み込みRustの分野は進歩が早く、できるだけ新しいプラットフォーム(Embedded-Rustチームが提供する`cortex-m`, `cortex-m-rt`, HALなど)を活用するようにした。

参考書籍として、、、、、、、




## 主な構成と機能


## プロジェクトの作成

### 前提とするハードウエア


### エントリーポイント、例外ハンドラ、初期化ルーチン(`cortex-m-rt`)

`cortex-m-rt`が提供する関数修飾マクロについて

#### `#[entry]`

必須。初期化ルーチン(慣習的に`crt0`と呼ばれる)がメモリを初期化をしたあと、内部的に、`fn main`が呼ばれ、`fn main`は`#[entry]`が付けられた関数を呼ぶ(内部的には`__cortex_m_rt_main`という関数になる)。

#### `#[pre_init]`

`#[pre_init]`が付けられた関数があれば、`crt0`が、`fn main`を呼ぶ前にその関数を呼ぶ。

`#[pre_init]`関数と`#[excption]`関数(後述)は、`unsafe`モードで実行されるので、追加の`unsafe`指定は不要。

#### `#[exception]`

次の名前の関数に、`#[exception]`をつければ、それは例外ハンドラとなり、該当する例外が発生したときに呼ばれる。もしユーザがこれらの例外ハンドラを作成しなければ`DefaultHandler`が呼ばれる。

* `DefaultHandler`
* `NonMaskableInt`
* `HardFault`
* `MemoryManagement` (a)
* `BusFault` (a)
* `UsageFault` (a)
* `SecureFault` (b)
* `SVCall`
* `DebugMonitor` (a)
* `PendSV`
* `SysTick`

(a) Not available on Cortex-M0 variants (thumbv6m-none-eabi)

(b) Only available on ARMv8-M


### Cortex-M0+(ARM-v6M,thumbv6m) で注意すること

組み込みRustに関する記事や書籍も多い。しかし、それらのほとんどがCortex-M3またはそれ以上のプロセッサを前提としている。Cortex-M3はARM-v7Mアーキテクチャ(thumbv7m)である。しかし、今回ターゲットとしているRP2040はCortex-M0+で、アーキテクチャはARM-v6Mである。いくつかの命令が実装されておらず、注意しなければならないことがおおい。

Raspberry Pi Pico2に搭載されているRP2350はCortex-M33デュアルコアであり、ほとんどの情報が適用できる。さらにRISC-Vコアも搭載されているので、そちらも試すことができる。素晴らしい。

#### アトミック命令


#### 即値命令


#### レジスタに積めるスタック



### RP2040で注意すること(SpinLock)

一般的なCortex-M0+コアと異なり、RP2040はCortex-M0+のデュアルコアである。

最も大きな違いは、割り込み禁止によるクリティカル・セクションが作れないこと。割り込み禁止はコアごとに動作する。一つのコアで割り込みを禁止しても、他方のコアは自由にクリティカルなメモリにアクセスできてしまう。

それに対する支援として、ハードウエアでSpinLockが提供されている。これは1ビットの値を持つレジスタで、SIO領域に32個存在し、双方のコアからアクセスできる。

それを使って両方のコアで有効なスピンロックを実装できる。

#### boot2

もうひとつ特徴的なものは`boot2`ブートローダだ。一般的なシステムでは、リセットアドレスからFLASHの先頭にジャンプし、ユーザコードをFLASHの先頭におけば、即座に実行される。

RP2040では、内蔵ROMは、外付けフラッシュの先頭にジャンプするが、そこには`boot2`と呼ばれるブートローダを書き込む。そして、`boot2`がユーザコードを実行する。通常は`boot2`はBSP(Board Support Package)クレートがハンドリングするが、自前で置き換えることもできる([boot-k](https://nkon.github.io/RP2040-Boot2/)プロジェクトを参照)。


## タスク切換え




### 特権モード


### 例外フレーム


### 関数呼び出し手順(AAPCS: Arm Architecture Procedure Call Standard)


#### 関数プロローグ、エピローグ、`naked`




## Mutexの実装


## SysTickハンドラ

Cortex-Mコアは、コアペリフェラルとしてSysTickというタイマをもっており、RTOSなどの基本タイマとして使われる。

SysTick割り込みを設定する。

SysTik割り込みは通常の割り込みではなく例外として処理される。

systick.rs(要点)

* Count型を定義し、`incr()`によって、外部に所有権を取得することなく、`wrapping_add()`を内部演算で処理する。
    + `Mutex`の実装である`UnsafeCell`の内部可変性を利用している。
* `init()`は`&mut SYST`とリロードカウンタの値を引数で取る。`SYST`はここに所有権をmoveするので他では使えなくなる(同じくSYSTを使っているcortex_m::delay::Delayは同時には使えない)。
* `static`変数として`SYSTICK_COUNT`を用意し、Mutex経由でアクセスする。
    + `SYSTICK_COUNT`の値を操作するときは`lock()`を取る→`MutexGuard`が帰る→`Deref`により透過的に中身にアクセスできる。
    + スコープが外れれば、ロックはドロップされる。
* `SYSTICK_COUNT`の値は外部で読めるようにする。
    + オブジェクト経由のアクセスではなく、`systick::count_get()`関数を呼ぶ。Rustらしくないが、こうするのが所有権問題を簡単に解決できる。
* 例外ハンドラでは`SYSTICK_COUNT`をインクリメントし、PendSVを呼ぶ(タスクスイッチが強制的に発生する)。


```rust
struct Count(u32);

impl Count {
    const fn new(value: u32) -> Self {
        Self(value)
    }
    fn incr(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

// Mutex::new, Count::new が const fn なので、static変数を初期化できる
static SYSTICK_COUNT: Mutex<Count> = Mutex::new(Count::new(0));

pub fn init(syst: &mut cortex_m::peripheral::SYST, reload: u32) {
    syst.set_clock_source(SystClkSource::Core);
    syst.set_reload(reload);
    syst.clear_current();
    syst.enable_counter();
    syst.enable_interrupt();
}

fn count_incr() {
    // lockを取って、UnsafeCell<>の中の値を操作する(mutでなくてもOK:内部可変性)
    SYSTICK_COUNT.lock().incr();
}

pub fn count_get() -> u32 {
    SYSTICK_COUNT.lock().0
}

#[exception]
fn SysTick() {
    info!("SysTick:{}", systick::count_get());
    systick::count_incr();
    SCB::set_pendsv();
}
```

呼び出し側

```rust
    let mut pac = pac::Peripherals::take().unwrap();        // pacの取得
    let mut core = pac::CorePeripherals::take().unwrap();   // coreの取得
    let mut watchdog = Watchdog::new(pac.WATCHDOG);         // watchdogの初期化

    let external_xtal_freq_hz = 12_000_000u32;              // RasPicoでは12MHzの外部クロックが接続されている
    let clocks = init_clocks_and_plls(                      // 内部クロックの初期化(デフォルトでは125MHz)
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    // コアクロックの周波数を確認
    info!("system clock = {}", clocks.system_clock.freq().to_kHz()); // 125000kHz = 125MHz

    // コア周波数が125MHzなので
    // リロード値を 125_000にしたら 1kHzでSysTickが割り込む。
    // リロード値を 125_000 * 100 にしたら 100msでSysTickが割り込む。
    // リロード値の最大は 0xff_ffff(24bit)=16_777_215=125_000*134.21772(ms)が最も遅い設定
    systick::init(&mut core.SYST, clocks.system_clock.freq().to_kHz() * 100); // SysTick = 100ms
```



## alloc::boxed::Box, Box::leak(), GlobalAlloc






## gdbの使い方


## 参考文献