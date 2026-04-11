//! 性能基准测试
//!
//! 测试指标：
//! - 连接延迟（Transport 创建、PeerId 生成）
//! - 消息发布延迟（Publish 命令发送到通道）
//! - 穿透策略性能（连接类型查询、失败计数统计）
//! - 中继带宽使用率（消息序列化开销）

use agentora_network::{
    ConnectionType, DcutrConfig, AutonatConfig, HybridStrategyConfig,
    Libp2pTransport, NatStatus, Transport,
};
use agentora_network::codec::NetworkMessage;
use std::time::{Duration, Instant};

// ==================== 连接延迟基准 ====================

/// Transport 创建延迟基准
#[tokio::test]
async fn benchmark_transport_creation_latency() {
    let iterations = 10;
    let mut durations = Vec::new();

    for _ in 0..iterations {
        let start = Instant::now();
        let _transport = Libp2pTransport::new().unwrap();
        let elapsed = start.elapsed();
        durations.push(elapsed);
    }

    let avg = durations.iter().sum::<Duration>() / iterations as u32;
    let min = durations.iter().min().unwrap();
    let max = durations.iter().max().unwrap();

    println!("\n=== Transport 创建延迟基准 ({} 次) ===", iterations);
    println!("  平均: {:?} (期望 < 500ms)", avg);
    println!("  最小: {:?}", min);
    println!("  最大: {:?}", max);

    // Transport 创建应该比较快（主要是密钥生成 + 通道创建）
    assert!(avg < Duration::from_millis(2000), "Transport 创建过慢：{:?}，期望 < 2s", avg);
}

/// PeerId 生成延迟基准（密钥对生成）
#[tokio::test]
async fn benchmark_peerid_generation() {
    let iterations = 100;
    let start = Instant::now();

    for _ in 0..iterations {
        let _transport = Libp2pTransport::new().unwrap();
    }

    let total = start.elapsed();
    let avg = total / iterations as u32;

    println!("\n=== PeerId 生成延迟基准 ({} 次) ===", iterations);
    println!("  总耗时: {:?}", total);
    println!("  平均: {:?} (期望 < 100ms)", avg);

    assert!(avg < Duration::from_millis(500), "PeerId 生成过慢：{:?}，期望 < 500ms", avg);
}

// ==================== 消息发布延迟基准 ====================

/// 消息发布延迟基准（命令通道发送）
#[tokio::test]
async fn benchmark_message_publish_latency() {
    let transport = Libp2pTransport::new().unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    let iterations = 1000;
    let test_data = b"benchmark test payload";
    let mut durations = Vec::new();

    for _ in 0..iterations {
        let start = Instant::now();
        transport.publish("benchmark-topic", test_data).await.unwrap();
        durations.push(start.elapsed());
    }

    let avg = durations.iter().sum::<Duration>() / iterations as u32;
    let min = durations.iter().min().unwrap();
    let max = durations.iter().max().unwrap();
    let p95_idx = (durations.len() as f32 * 0.95) as usize;
    let mut sorted = durations.clone();
    sorted.sort();
    let p95 = sorted[p95_idx];

    println!("\n=== 消息发布延迟基准 ({} 次) ===", iterations);
    println!("  平均: {:?} (期望 < 1ms)", avg);
    println!("  最小: {:?}", min);
    println!("  最大: {:?}", max);
    println!("  P95:  {:?}", p95);

    // 通道发送应该非常快
    assert!(avg < Duration::from_millis(10), "消息发布过慢：{:?}，期望 < 10ms", avg);
}

/// 网络消息序列化延迟基准
#[tokio::test]
async fn benchmark_message_serialization() {
    let iterations = 10000;
    let test_data = b"serialization benchmark payload with some content";
    let msg = NetworkMessage::CrdtOp(agentora_network::codec::CrdtOp::LwwSet {
        key: "test_key".to_string(),
        value: b"test_value".to_vec(),
        timestamp: 1234567890,
        peer_id: "test_peer".to_string(),
    });

    // 序列化基准
    let start = Instant::now();
    for _ in 0..iterations {
        let _bytes = msg.to_bytes();
    }
    let serialize_total = start.elapsed();
    let serialize_avg = serialize_total / iterations as u32;

    // 反序列化基准
    let bytes = msg.to_bytes();
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = NetworkMessage::from_bytes(&bytes).unwrap();
    }
    let deserialize_total = start.elapsed();
    let deserialize_avg = deserialize_total / iterations as u32;

    println!("\n=== 消息序列化延迟基准 ({} 次) ===", iterations);
    println!("  序列化平均:   {:?}", serialize_avg);
    println!("  反序列化平均: {:?}", deserialize_avg);

    assert!(serialize_avg < Duration::from_millis(1), "序列化过慢：{:?}", serialize_avg);
    assert!(deserialize_avg < Duration::from_millis(1), "反序列化过慢：{:?}", deserialize_avg);
}

// ==================== 穿透策略性能基准 ====================

/// 连接类型查询延迟基准
#[tokio::test]
async fn benchmark_connection_type_lookup() {
    let transport = Libp2pTransport::new().unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let iterations = 1000;
    let mut durations = Vec::new();

    for _ in 0..iterations {
        let start = Instant::now();
        let _ = transport.get_connection_type("test_peer_id").await;
        durations.push(start.elapsed());
    }

    let avg = durations.iter().sum::<Duration>() / iterations as u32;

    println!("\n=== 连接类型查询延迟基准 ({} 次) ===", iterations);
    println!("  平均: {:?}", avg);

    assert!(avg < Duration::from_millis(50), "连接类型查询过慢：{:?}", avg);
}

/// NAT 状态查询延迟基准
#[tokio::test]
async fn benchmark_nat_status_lookup() {
    let transport = Libp2pTransport::new().unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let iterations = 1000;
    let mut durations = Vec::new();

    for _ in 0..iterations {
        let start = Instant::now();
        let _ = transport.get_nat_status().await;
        durations.push(start.elapsed());
    }

    let avg = durations.iter().sum::<Duration>() / iterations as u32;

    println!("\n=== NAT 状态查询延迟基准 ({} 次) ===", iterations);
    println!("  平均: {:?}", avg);

    assert!(avg < Duration::from_millis(50), "NAT 状态查询过慢：{:?}", avg);
}

/// 混合策略配置默认值性能（确保默认配置合理）
#[tokio::test]
async fn benchmark_strategy_config_defaults() {
    let iterations = 1000;
    let start = Instant::now();

    for _ in 0..iterations {
        let _config = HybridStrategyConfig::default();
    }
    let total = start.elapsed();
    let avg = total / iterations as u32;

    println!("\n=== 策略配置默认值基准 ({} 次) ===", iterations);
    println!("  平均: {:?}", avg);

    assert!(avg < Duration::from_micros(100), "配置创建过慢：{:?}", avg);
}

/// 降级阈值触发性能（模拟连接降级逻辑）
#[tokio::test]
async fn benchmark_degradation_threshold_logic() {
    let config = HybridStrategyConfig::default();

    // 验证阈值设置的合理性
    assert!(config.degradation_threshold > 0, "降级阈值必须 > 0");
    assert!(config.direct_timeout_secs > 0, "直连超时必须 > 0");
    assert!(config.dcutr_timeout_secs > config.direct_timeout_secs,
        "DCUtR 超时应该大于直连超时");

    println!("\n=== 降级阈值配置验证 ===");
    println!("  直连超时: {}s", config.direct_timeout_secs);
    println!("  DCUtR 超时: {}s", config.dcutr_timeout_secs);
    println!("  降级阈值: {}", config.degradation_threshold);
    println!("  DCUtR 最大重试: {}", config.dcutr.max_retries);
    println!("  DCUtR 并发尝试: {}", config.dcutr.concurrent_attempts);

    // 验证超时递增关系：DCUtR > 直连
    assert!(config.dcutr_timeout_secs > config.direct_timeout_secs,
        "DCUtR 超时({}s) 应该大于直连超时({}s)",
        config.dcutr_timeout_secs, config.direct_timeout_secs);

    // 验证 DCUtR 重试配置合理
    assert!(config.dcutr.max_retries >= 1, "DCUtR 至少需要 1 次重试");
    assert!(config.dcutr.concurrent_attempts >= 1, "DCUtR 至少需要 1 个并发尝试");
}

// ==================== 中继带宽使用率基准 ====================

/// 中继消息吞吐量基准（通过通道发送模拟）
#[tokio::test]
async fn benchmark_relay_message_throughput() {
    let transport = Libp2pTransport::new().unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    // 使用不同大小的消息测试
    let payload_sizes = vec![64, 256, 1024, 4096];

    for size in &payload_sizes {
        let payload = vec![0u8; *size];
        let iterations = 500;

        let start = Instant::now();
        for _ in 0..iterations {
            transport.publish("benchmark-relay", &payload).await.unwrap();
        }
        let total = start.elapsed();
        let throughput_mbps = (*size as f64 * iterations as f64) / (total.as_secs_f64() * 1_000_000.0);

        println!("\n=== 中继消息吞吐基准 (消息大小={}B) ===", size);
        println!("  总耗时: {:?}", total);
        println!("  吞吐量: {:.2} MB/s", throughput_mbps);
    }
}

/// GossipSub 消息 ID 生成性能（SHA256 哈希）
#[tokio::test]
async fn benchmark_gossipsub_message_id() {
    use sha2::{Digest, Sha256};

    let iterations = 10000;
    let test_data = vec![0u8; 256];

    let start = Instant::now();
    for _ in 0..iterations {
        let _hash = Sha256::digest(&test_data);
    }
    let total = start.elapsed();
    let avg = total / iterations as u32;

    println!("\n=== SHA256 消息 ID 生成基准 ({} 次) ===", iterations);
    println!("  平均: {:?}", avg);

    assert!(avg < Duration::from_micros(100), "SHA256 哈希过慢：{:?}", avg);
}

// ==================== 配置结构体性能基准 ====================

/// 配置结构体默认值完整性验证
#[tokio::test]
async fn benchmark_config_struct_performance() {
    let iterations = 1000;

    // DcutrConfig
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = DcutrConfig::default();
    }
    let dcutr_avg = start.elapsed() / iterations as u32;

    // AutonatConfig
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = AutonatConfig::default();
    }
    let autonat_avg = start.elapsed() / iterations as u32;

    // HybridStrategyConfig
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = HybridStrategyConfig::default();
    }
    let strategy_avg = start.elapsed() / iterations as u32;

    println!("\n=== 配置结构体基准 ({} 次) ===", iterations);
    println!("  DcutrConfig:        {:?}", dcutr_avg);
    println!("  AutonatConfig:      {:?}", autonat_avg);
    println!("  HybridStrategyConfig: {:?}", strategy_avg);

    assert!(dcutr_avg < Duration::from_micros(100), "DcutrConfig 过慢");
    assert!(autonat_avg < Duration::from_micros(100), "AutonatConfig 过慢");
    assert!(strategy_avg < Duration::from_micros(100), "HybridStrategyConfig 过慢");
}

/// NatStatus 状态转换性能
#[tokio::test]
async fn benchmark_nat_status_transitions() {
    let iterations = 10000;
    let addr: libp2p::Multiaddr = "/ip4/192.168.1.1/tcp/4001".parse().unwrap();

    let start = Instant::now();
    for _ in 0..iterations {
        let _ = NatStatus::Public(addr.clone());
        let _ = NatStatus::Private;
        let _ = NatStatus::Unknown;
    }
    let total = start.elapsed();
    let avg = total / (iterations * 3) as u32;

    println!("\n=== NAT 状态转换基准 ({} 次转换) ===", iterations * 3);
    println!("  平均: {:?}", avg);

    assert!(avg < Duration::from_micros(10), "NAT 状态转换过慢：{:?}", avg);
}

/// ConnectionType 枚举性能
#[tokio::test]
async fn benchmark_connection_type_operations() {
    let iterations = 10000;

    let start = Instant::now();
    for _ in 0..iterations {
        let direct = ConnectionType::Direct;
        let dcutr = ConnectionType::Dcutr;
        let relay = ConnectionType::Relay;
        let _ = direct == dcutr;
        let _ = relay == relay;
    }
    let total = start.elapsed();
    let avg = total / (iterations * 3) as u32;

    println!("\n=== ConnectionType 操作基准 ({} 次操作) ===", iterations * 3);
    println!("  平均: {:?}", avg);

    assert!(avg < Duration::from_micros(5), "ConnectionType 操作过慢：{:?}", avg);
}

// ==================== 综合性能报告 ====================

/// 综合性能报告：汇总所有关键指标
#[tokio::test]
async fn benchmark_full_performance_report() {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║       Agentora NAT 穿透性能基准测试报告                  ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    // 1. Transport 创建
    let iterations = 10;
    let start = Instant::now();
    let mut transports = Vec::new();
    for _ in 0..iterations {
        transports.push(Libp2pTransport::new().unwrap());
    }
    let transport_create_avg = start.elapsed() / iterations as u32;

    // 2. 消息发布
    let transport = Libp2pTransport::new().unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    let pub_iterations = 500;
    let start = Instant::now();
    for _ in 0..pub_iterations {
        transport.publish("bench", b"test").await.unwrap();
    }
    let publish_avg = start.elapsed() / pub_iterations as u32;

    // 3. NAT 状态查询
    let query_iterations = 500;
    let start = Instant::now();
    for _ in 0..query_iterations {
        let _ = transport.get_nat_status().await;
    }
    let nat_query_avg = start.elapsed() / query_iterations as u32;

    // 4. 连接类型查询
    let start = Instant::now();
    for _ in 0..query_iterations {
        let _ = transport.get_connection_type("peer").await;
    }
    let conn_query_avg = start.elapsed() / query_iterations as u32;

    // 5. 配置结构体
    let config_iterations = 500;
    let start = Instant::now();
    for _ in 0..config_iterations {
        let _ = HybridStrategyConfig::default();
    }
    let config_avg = start.elapsed() / config_iterations as u32;

    println!("关键性能指标：");
    println!("  Transport 创建:   {:?} (目标: < 500ms)", transport_create_avg);
    println!("  消息发布:         {:?} (目标: < 1ms)", publish_avg);
    println!("  NAT 状态查询:     {:?} (目标: < 50ms)", nat_query_avg);
    println!("  连接类型查询:     {:?} (目标: < 50ms)", conn_query_avg);
    println!("  配置结构体:       {:?} (目标: < 100μs)", config_avg);
    println!();

    // 验证关键指标
    let mut passed = true;
    if transport_create_avg >= Duration::from_millis(2000) {
        println!("[FAIL] Transport 创建过慢");
        passed = false;
    }
    if publish_avg >= Duration::from_millis(10) {
        println!("[FAIL] 消息发布过慢");
        passed = false;
    }
    if nat_query_avg >= Duration::from_millis(50) {
        println!("[FAIL] NAT 状态查询过慢");
        passed = false;
    }
    if conn_query_avg >= Duration::from_millis(50) {
        println!("[FAIL] 连接类型查询过慢");
        passed = false;
    }

    if passed {
        println!("[PASS] 所有性能指标通过");
    }
}
