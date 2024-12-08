use core::marker::PhantomData;

// TODO: 名前をkernelモジュールに変更する。
// TODO: スケジューラ、タスクハンドラ関係をまとめる。

pub struct SystemData {
    marker: PhantomData<u32>,
}

impl SystemData {
    pub fn new() -> Self {
        SystemData {
            marker: PhantomData,
        }
    }
}

impl Default for SystemData {
    fn default() -> Self {
        Self::new()
    }
}
