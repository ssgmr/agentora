# ZCache 使用指南

## 初始化

```python
from sofapy_base.app.layotto_manager import get_layotto_manager

manager = get_layotto_manager()
zcache = manager.zcache_manager.get_zcache("cache_instance_name")
```

## String 操作

```python
# 连通性测试
zcache.echo("test")

# 获取值
zcache.get("key")

# 设置值
zcache.set("key", "value")

# 设置值并指定过期时间（秒）
zcache.setex("key", 60, "value")

# 设置值并指定过期时间（毫秒）
zcache.psetex("key", 1000, "value")

# 仅当 key 不存在时才设置
zcache.setnx("key", "value")

# 设置新值并返回旧值
zcache.getset("key", "new_value")

# 从指定偏移覆盖值
zcache.setrange("key", 6, "value")

# 获取指定范围的值
zcache.getrange("key", 0, 10)

# 追加值到键
zcache.append("key", "value")

# 批量设置
zcache.mset({"k1": "v1", "k2": "v2"})

# 批量获取
zcache.mget("k1", "k2")

# 数值自增 1
zcache.incr("key")

# 数值自增指定值
zcache.incrby("key", 5)

# 浮点数自增
zcache.incrbyfloat("key", 1.5)

# 数值自减 1
zcache.decr("key")

# 数值自减指定值
zcache.decrby("key", 5)

# 获取值长度
zcache.strlen("key")

# 删除键
zcache.delete("key1", "key2")

# 检查键是否存在
zcache.exists("key")
```

### CZone String API

```python
czone_cache.zone_get("CZ00D", "key")
czone_cache.zone_set("CZ00D", "key", "value")
czone_cache.zone_setex("CZ00D", "key", 10, "value")
czone_cache.zone_setnx("CZ00D", "key", "value")
czone_cache.zone_getset("CZ00D", "key", "value")
czone_cache.zone_setrange("CZ00D", "key", 6, "value")
czone_cache.zone_getrange("CZ00D", "key", 0, 10)
czone_cache.zone_append("CZ00D", "key", "value")
czone_cache.zone_mset("CZ00D", {"k1": "v1", "k2": "v2"})
czone_cache.zone_mget("CZ00D", "k1", "k2")
czone_cache.zone_incr("CZ00D", "key")
czone_cache.zone_incrby("CZ00D", "key", 5)
czone_cache.zone_incrbyfloat("CZ00D", "key", 1.5)
czone_cache.zone_decr("CZ00D", "key")
czone_cache.zone_decrby("CZ00D", "key", 5)
czone_cache.zone_strlen("CZ00D", "key")
czone_cache.zone_delete("CZ00D", "key1", "key2")
czone_cache.zone_exists("CZ00D", "key")
```

## Hash 操作

```python
# 设置 hash 字段
zcache.hset("hash_name", "field", "value")

# 批量设置 hash 字段
zcache.hset("hash_name", mapping={"f1": "v1", "f2": "v2"})

# 通过列表设置
zcache.hset("hash_name", items=["f1", "v1", "f2", "v2"])

# 字段不存在时才设置
zcache.hsetnx("hash_name", "field", "value")

# 获取 hash 字段值
zcache.hget("hash_name", "field")

# 批量设置（旧 API）
zcache.hmset("hash_name", {"f1": "v1"})

# 批量获取字段
zcache.hmget("hash_name", "f1", "f2")

# 获取所有字段
zcache.hgetall("hash_name")

# 删除字段
zcache.hdel("hash_name", "f1", "f2")

# 检查字段是否存在
zcache.hexists("hash_name", "field")

# 字段值整数自增
zcache.hincrby("hash_name", "field", 5)

# 字段值浮点自增
zcache.hincrbyfloat("hash_name", "field", 1.5)

# 获取字段数量
zcache.hlen("hash_name")

# 获取所有字段名
zcache.hkeys("hash_name")

# 获取所有字段值
zcache.hvals("hash_name")

# 遍历 hash
zcache.hscan("hash_name", match="key*")
```

### CZone Hash API

```python
czone_cache.zone_hset("CZ00D", "hash_name", "field", "value")
czone_cache.zone_hset("CZ00D", "hash_name", mapping={"f1": "v1"})
czone_cache.zone_hsetnx("CZ00D", "hash_name", "field", "value")
czone_cache.zone_hget("CZ00D", "hash_name", "field")
czone_cache.zone_hmset("CZ00D", "hash_name", {"f1": "v1"})
czone_cache.zone_hmget("CZ00D", "hash_name", "f1", "f2")
czone_cache.zone_hgetall("CZ00D", "hash_name")
czone_cache.zone_hdel("CZ00D", "hash_name", "f1", "f2")
czone_cache.zone_hexists("CZ00D", "hash_name", "field")
czone_cache.zone_hincrby("CZ00D", "hash_name", "field", 5)
czone_cache.zone_hincrbyfloat("CZ00D", "hash_name", "field", 1.5)
czone_cache.zone_hlen("CZ00D", "hash_name")
czone_cache.zone_hkeys("CZ00D", "hash_name")
czone_cache.zone_hvals("CZ00D", "hash_name")
czone_cache.zone_hscan("CZ00D", "hash_name", match="key*")
```

## List 操作

```python
# 左侧推入值
zcache.lpush("list_name", "value")

# list 存在时才左侧推入
zcache.lpushx("list_name", "value")

# 右侧推入值
zcache.rpush("list_name", "value")

# list 存在时才右侧推入
zcache.rpushx("list_name", "value")

# 获取范围元素
zcache.lrange("list_name", 0, -1)

# 左侧弹出元素
zcache.lpop("list_name")

# 右侧弹出元素
zcache.rpop("list_name")

# 移除指定值
zcache.lrem("list_name", 1, "value")

# 设置指定索引值
zcache.lset("list_name", 0, "value")

# 裁剪 list
zcache.ltrim("list_name", 0, 5)

# 获取指定索引值
zcache.lindex("list_name", 0)

# 插入值
zcache.linsert("list_name", "BEFORE", "pivot", "value")

# 获取 list 长度
zcache.llen("list_name")
```

### CZone List API

```python
czone_cache.zone_lpush("CZ00D", "list_name", "value")
czone_cache.zone_lpushx("CZ00D", "list_name", "value")
czone_cache.zone_rpush("CZ00D", "list_name", "value")
czone_cache.zone_rpushx("CZ00D", "list_name", "value")
czone_cache.zone_lrange("CZ00D", "list_name", 0, -1)
czone_cache.zone_lpop("CZ00D", "list_name")
czone_cache.zone_rpop("CZ00D", "list_name")
czone_cache.zone_lrem("CZ00D", "list_name", 1, "value")
czone_cache.zone_lset("CZ00D", "list_name", 0, "value")
czone_cache.zone_ltrim("CZ00D", "list_name", 0, 5)
czone_cache.zone_lindex("CZ00D", "list_name", 0)
czone_cache.zone_linsert("CZ00D", "list_name", "BEFORE", "pivot", "value")
czone_cache.zone_llen("CZ00D", "list_name")
```

## Set 操作

```python
# 添加成员
zcache.sadd("set_name", "m1", "m2")

# 移除成员
zcache.srem("set_name", "m1")

# 随机弹出成员
zcache.spop("set_name")

# 获取所有成员
zcache.smembers("set_name")

# 检查成员是否存在
zcache.sismember("set_name", "member")

# 随机获取成员（不删）
zcache.srandmember("set_name")

# 获取成员数量
zcache.scard("set_name")

# 遍历 set
zcache.sscan("set_name", match="value*")
```

### CZone Set API

```python
czone_cache.zone_sadd("CZ00D", "set_name", "m1", "m2")
czone_cache.zone_srem("CZ00D", "set_name", "m1")
czone_cache.zone_spop("CZ00D", "set_name")
czone_cache.zone_smembers("CZ00D", "set_name")
czone_cache.zone_sismember("CZ00D", "set_name", "member")
czone_cache.zone_srandmember("CZ00D", "set_name")
czone_cache.zone_scard("CZ00D", "set_name")
czone_cache.zone_sscan("CZ00D", "set_name", match="value*")
```

## ZSet 操作

```python
# 添加成员
zcache.zadd("zset_name", {"one": 1, "two": 2})

# 移除成员
zcache.zrem("zset_name", "one")

# 增加成员分数
zcache.zincrby("zset_name", 5, "one")

# 获取排名（升序）
zcache.zrank("zset_name", "one")

# 获取排名（降序）
zcache.zrevrank("zset_name", "one")

# 获取成员数量
zcache.zcard("zset_name")

# 获取成员分数
zcache.zscore("zset_name", "one")

# 统计分数范围内成员
zcache.zcount("zset_name", 0, 100)

# 获取范围成员（升序）
zcache.zrange("zset_name", 0, -1, withscores=True)

# 获取范围成员（降序）
zcache.zrevrange("zset_name", 0, -1, withscores=True)

# 按分数范围获取
zcache.zrangebyscore("zset_name", 0, 100, withscores=True)

# 按分数范围获取（降序）
zcache.zrevrangebyscore("zset_name", 100, 0, withscores=True)

# 按排名范围移除
zcache.zremrangebyrank("zset_name", 0, 5)

# 按分数范围移除
zcache.zremrangebyscore("zset_name", 0, 100)
```

### CZone ZSet API

```python
czone_cache.zone_zadd("CZ00D", "zset_name", {"one": 1, "two": 2})
czone_cache.zone_zrem("CZ00D", "zset_name", "one")
czone_cache.zone_zincrby("CZ00D", "zset_name", 5, "one")
czone_cache.zone_zrank("CZ00D", "zset_name", "one")
czone_cache.zone_zrevrank("CZ00D", "zset_name", "one")
czone_cache.zone_zcard("CZ00D", "zset_name")
czone_cache.zone_zscore("CZ00D", "zset_name", "one")
czone_cache.zone_zcount("CZ00D", "zset_name", 0, 100)
czone_cache.zone_zrange("CZ00D", "zset_name", 0, -1, withscores=True)
czone_cache.zone_zrevrange("CZ00D", "zset_name", 0, -1, withscores=True)
czone_cache.zone_zrangebyscore("CZ00D", "zset_name", 0, 100, withscores=True)
czone_cache.zone_zrevrangebyscore("CZ00D", "zset_name", 100, 0, withscores=True)
czone_cache.zone_zremrangebyrank("CZ00D", "zset_name", 0, 5)
czone_cache.zone_zremrangebyscore("CZ00D", "zset_name", 0, 100)
```

## 过期与元数据操作

```python
# 设置过期时间
zcache.expire("key", 60)

# 设置过期时间点
zcache.expireat("key", datetime.now())

# 获取剩余过期时间（秒）
zcache.ttl("key")

# 获取剩余过期时间（毫秒）
zcache.pttl("key")

# 获取键类型
zcache.type("key")
```

### CZone 元数据 API

```python
czone_cache.zone_expire("CZ00D", "key", 60)
czone_cache.zone_expireat("CZ00D", "key", datetime.now())
czone_cache.zone_ttl("CZ00D", "key")
czone_cache.zone_pttl("CZ00D", "key")
czone_cache.zone_type("CZ00D", "key")
```
