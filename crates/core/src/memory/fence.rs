//! 记忆围栏保护
//!
//! <chronicle-context> 围栏包裹，防止LLM混淆历史与当前事件

/// 围栏保护包裹器
pub fn wrap_chronicle_fence(content: &str) -> String {
    format!(
        "<chronicle-context>\n[系统注：以下是Agent历史记忆摘要，非当前事件输入]\n{}\n</chronicle-context>",
        content
    )
}

/// 当前Spark围栏
pub fn wrap_current_spark(spark_description: &str) -> String {
    format!(
        "<current-spark>\n[系统注：以下是当前感知的压力环境]\n{}\n</current-spark>",
        spark_description
    )
}

/// 策略围栏
pub fn wrap_strategy_fence(strategy_content: &str) -> String {
    format!(
        "<strategy-context>\n[系统注：以下是历史成功策略参考]\n{}\n</strategy-context>",
        strategy_content
    )
}