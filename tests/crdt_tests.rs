//! 单元测试 - CRDT
//!
//! 测试LWW-Register、G-Counter、OR-Set合并正确性

use agentora_sync::lww::LwwRegister;
use agentora_sync::gcounter::GCounter;
use agentora_sync::orset::OrSet;
use agentora_sync::PeerId;  // 使用agentora_sync的PeerId

#[test]
fn test_lww_register_new() {
    let register = LwwRegister::new("value1".to_string(), 100, PeerId::new("peer1"));
    assert_eq!(register.get(), "value1");
}

#[test]
fn test_lww_register_set_higher_timestamp() {
    let mut register = LwwRegister::new("value1".to_string(), 100, PeerId::new("peer1"));

    // 更高timestamp更新值
    register.set("value2".to_string(), 200, PeerId::new("peer2"));

    assert_eq!(register.get(), "value2");
    assert_eq!(register.timestamp(), 200);
}

#[test]
fn test_lww_register_merge() {
    let mut local = LwwRegister::new("value1".to_string(), 100, PeerId::new("peer1"));
    let remote = LwwRegister::new("value2".to_string(), 200, PeerId::new("peer2"));

    local.merge(&remote);

    // 应取较高timestamp的值
    assert_eq!(local.get(), "value2");
}

#[test]
fn test_lww_register_peer_id_tie_breaker() {
    let mut register = LwwRegister::new("value1".to_string(), 100, PeerId::new("peer1"));

    // 相同timestamp，peer_id大的获胜
    register.set("value2".to_string(), 100, PeerId::new("peer2"));

    // peer2 > peer1 字符串比较
    assert_eq!(register.get(), "value2");
}

#[test]
fn test_g_counter_new() {
    let counter = GCounter::new();
    assert_eq!(counter.total(), 0);
}

#[test]
fn test_g_counter_increment() {
    let mut counter = GCounter::new();
    let peer = PeerId::new("peer1");

    counter.increment(&peer, 10);
    counter.increment(&peer, 5);

    assert_eq!(counter.total(), 15);
    assert_eq!(counter.local_count(&peer), 15);
}

#[test]
fn test_g_counter_merge() {
    let mut local = GCounter::new();
    let mut remote = GCounter::new();

    let peer1 = PeerId::new("peer1");
    let peer2 = PeerId::new("peer2");

    local.increment(&peer1, 10);
    remote.increment(&peer2, 20);
    remote.increment(&peer1, 5);  // peer1在remote只有5

    local.merge(&remote);

    // peer1取max: max(10, 5) = 10
    // peer2取max: max(0, 20) = 20
    assert_eq!(local.total(), 30);
    assert_eq!(local.local_count(&peer1), 10);
    assert_eq!(local.local_count(&peer2), 20);
}

#[test]
fn test_or_set_new() {
    let set: OrSet<String> = OrSet::new();
    assert_eq!(set.elements().len(), 0);
}

#[test]
fn test_or_set_add() {
    let mut set: OrSet<String> = OrSet::new();
    let peer = PeerId::new("peer1");

    set.add("element1".to_string(), &peer, 1);

    assert!(set.contains(&"element1".to_string()));
    assert_eq!(set.elements().len(), 1);
}

#[test]
fn test_or_set_remove() {
    let mut set: OrSet<String> = OrSet::new();
    let peer = PeerId::new("peer1");

    set.add("element1".to_string(), &peer, 1);
    set.remove(&"element1".to_string());

    assert!(!set.contains(&"element1".to_string()));
}

#[test]
fn test_or_set_merge_add_wins() {
    let mut local: OrSet<String> = OrSet::new();
    let mut remote: OrSet<String> = OrSet::new();

    let peer1 = PeerId::new("peer1");
    let peer2 = PeerId::new("peer2");

    local.add("element1".to_string(), &peer1, 1);
    remote.add("element1".to_string(), &peer2, 2);

    local.merge(&remote);

    // 并发添加，两个都保留（不同tag）
    assert!(local.contains(&"element1".to_string()));
}