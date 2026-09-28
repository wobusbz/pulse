// `diagnostics/` 是一个「命名空间目录」：后续的故障诊断能力（事件日志、崩溃上报…）
// 都会作为同级文件加进来，所以这里的内部模块名与目录同名是有意为之。
// `clippy::module_inception` 只是一条样式 lint，不影响行为，故显式允许。
#[allow(clippy::module_inception)]
pub(crate) mod diagnostics;
