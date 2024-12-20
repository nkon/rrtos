---
layout: post
title: Rust Cheet Sheet(アトミック操作)
category: blog
tags: rust atomic mutex
---

Rustのアトミック操作についての自分のためのまとめ。
『[詳解 Rustアトミック操作とロック](https://www.oreilly.co.jp/books/9784814400515/)』の読書メモ。

# `Box`

値をヒープに確保する。

* `Box::leak()`:確保された領域を開放しない。`'static`と互換性があるプログラム終了までのライフタイムを持つ。

# `Rc`

参照カウント。シングルスレッドでしか使えない。`Arc`より軽い。

* `clone()`しても中身は複製されない。

# `Arc`

「アトミック」参照カウント。他のスレッドに移動できる。CPUがアトミック操作をサポートしている必要がある。`Rc`よりも重い。

# `Cell`

内部可変性を提供する。

* シングルスレッドしか使えない
* 値を変更するときは、
    1. `.take()`で、現在の値を得て、`Cell`の中身は空に置き換える。
    2. 得られた値を変更して、
    3. `.set()`で変更された値を`Cell`に書き戻す。

# `RefCell`

参照カウンタを使った`Cell`。`Cell`よりも重い。

* シングルスレッドしか使えない
* 他にだれも可変借用していなければ、可変借用できる。
    + 可変借用を使えば、内容を直接変更できる。
* 普遍借用は複数できる。


# `UnsafeCell`

* `Cell`や`Mutex`の実装に使われる。
* `.get()`で生ポインタを取ることができるが、それは`unsafe`内で扱う必要がある。

# `Send`

トレイト。別のスレッドに送ることができる。

# `Sync`

* トレイト。別のスレッドと共有することができる。
* `&T`が`Send`ならば`T`は`Sync`。

例

* プリミティブ型(`u32`, `bool`, `str`など)は`Send`かつ`Sync`。
* すべてのフィールドが`Send`かつ`Sync`な構造体は`Send`かつ`Sync`(トレイトが自動で実装される)。
    + トレイトを自動で実装したくない場合は、`PhantomData<T>`のTに`Send`,`Sync`でない型を含めたメンバーをもたせる。`PhantomData`はコンパイラの型推論に使われるだけで、実際のメモリを消費しない。 
* `Arc<i32>`は`Send`。`Rc<i32>`は`Send`ではない。
* `Cell<i32>`は`Send`だが`Sync`ではない。
* 明示的に実装したい場合は`unsafe impl Send for X{}`, `unsafe impl Sync for X{}`と実装することができる。`Send`, `Sync`したときの安全性はコンパイラは保証してくれず、プログラマが保証しなければならないので`unsafe`。

# ロック: `Mutex`と`RwLock`

* `Mutex`はロックされていない状態とロックされている状態を取る。
    + ロックされているときは排他的なアクセス(読み書き)が可能。
    + `lock()`は、ロックが取れるまで待ち続ける。
        - `lock()`はロックが取れたら`MutexGuard`を返す。`Deref`, `DerefMut`トレイトが実装されてるので中のデータに透過的にアクセスできる。
    + `try_lock()`はロックが取れたら`Result<MutexGuard<T>>`を返し、取れなかったら`Err`を返す。 
    + ロックがスコープを外れて`Drop`したときに自動でアンロックされる。
        - スコープが終わらないうちに`Drop`したいときは`drop()`を使う。
        - `list.lock().unwrap().push(2);`のイデオムは、その場でロックを取得してメンバ関数を実行し、その行でロックをドロップする。
    + `get_mut()`はロックを取ったうえでミュータブルな参照を返す。
    + `into_inner()`はMutexを消費する。つまり、所有権を取得して中身を返す。`Mutex`による保護は失われ、以後、他のスレッドは`lock()`することができない。
* `RwLock`はロックしたときに、共有参照(`&T`)と排他参照(`&mut T`)を返す。
    + `read()`はリードロック(共有参照)
    + `write()`はライトロック(排他参照)
    + `RefCell`のマルチスレッド版

`Mutex`がドロップするスコープについて注意が必要。

```rust
    list.lock().unwrap().push(2);   // ロックはこの行でドロップされる
```
```rust
    if let Some(item) = list.lock().unwrap().pop() {
        process_item(item);
    }                                                   // ロックはこの行まで確保されたまま
```
```rust
    if list.lock().unwrap().pop() == Some(1) {          // ロックはこの行でドロップされる
        do_something;
    }
```
このように書けば良い。
```rust
    let item = list.lock().unwrap().pop();              // ロックはこの行でドロップされる
    if let Some(item) = item {
        process_item(item);
    }
```

# アトミック操作

* `load()`, `store()`操作はCortex-M0+でもサポートされている。
* `fetch_add()`, `compare_exchange()`操作はCortex-M0+ではサポートされていない。
    + Cortex-M3以上ではサポートされている。
* アトミック変数は内部可変性を持つ。
    + `mut`でなくても、`store()`操作が可能。
    + スレッド間で共有可能。`'static`にすることもできる。
* アトミック変数は`Copy`を実装していない。

## メモリ・オーダリング

* `Relaxed`: 一つのアトミック変数に対する操作順序が一貫している。複数の変数の場合は順序は保証されない。
* `Release`/`Acquire`: `Release`ストアが、`Acquire`ロードに先行する。つまり、`Release`より先の現象は`Acqire`より後で観測できる。
* `AcqRel`: `Acquire`と`Release`の組み合わせ。

