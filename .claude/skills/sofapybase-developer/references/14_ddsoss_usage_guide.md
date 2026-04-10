# DDSOSS 使用指南

DDSOSS 是蚂蚁内部的对象存储服务，支持对象上传下载、追加上传、标签管理、ACL 控制、签名 URL、分片上传等功能。

## 初始化

```python
from sofapy_base.app.layotto_manager import get_layotto_manager

manager = get_layotto_manager()
bucket = manager.get_oss_bucket(bucket="bucket_name")
```

## 基础对象操作

```python
# 上传对象（字符串或字节）
bucket.put_object("python-sdk.txt", "Hello OSS")
bucket.put_object("python-sdk2.txt", b"hello, python")

# 下载对象
content = bucket.get_object("python-sdk.txt")

# 下载对象到本地文件
bucket.get_object_to_file("python-sdk.txt", "python-sdk-local.txt")

# 从本地文件上传
bucket.put_object_from_file("python/python-sdk.txt", "python-sdk-local.txt")

# 检查对象是否存在
exist_result = bucket.is_object_exist("test_is_object_exist.txt")

# 获取对象元数据
head_result = bucket.head_object("test_head.txt")

# 删除单个对象
bucket.delete_object("python-sdk.txt")

# 批量删除对象
bucket.batch_delete_objects(["python-sdk2.txt", "python-sdk3.txt", "python/python-sdk.txt"])

# 复制对象
bucket.copy_object(bucket.bucket, "test_zz.txt", "test_zz_copy.txt")
```

## 追加上传

```python
# 追加上传（从指定位置开始）
bucket.append_object("test_append.txt", 0, b"test")
```

## 对象标签操作

```python
# 设置对象标签
bucket.put_object_tagging("test_put_object_tagging.txt", {"antsys": "true"})

# 获取对象标签
tagging_result = bucket.get_object_tagging("test_put_object_tagging.txt")

# 删除对象标签
bucket.delete_object_tagging("test_put_object_tagging.txt")
```

## 对象 ACL 操作

```python
# 设置对象 ACL
bucket.put_object_canned_acl("test_put_object_canned_acl.txt", "public-read")

# 获取对象 ACL
acl_result = bucket.get_object_canned_acl("test_put_object_canned_acl.txt")
```

## 对象列表操作

```python
# 列出对象
result = bucket.list_objects("", "", "")
for obj in result.object_list:
    print(obj.key)

# 使用迭代器遍历对象
from layotto.core.oss_module import ObjectIterator

for obj in ObjectIterator(bucket):
    print(obj.key)
```

## 签名 URL 操作

```python
from layotto import HTTPMethod

# 生成签名 URL（用于临时访问）
url = bucket.sign_url(HTTPMethod.HTTPGet, "python-sdk.txt", 60)
```

## 分片上传操作

```python
from layotto.core.proto.oss_pb2 import CompletedPart, CompletedMultipartUpload

# 初始化分片上传
create_result = bucket.create_multipart_upload("test_create_multipart_upload.txt")
upload_id = create_result.upload_id

# 上传分片
upload_result = bucket.upload_part("test_create_multipart_upload.txt", upload_id, 1, b"test")
etag = upload_result.etag

# 列出已上传的分片
list_parts_result = bucket.list_parts("test_create_multipart_upload.txt", upload_id)

# 列出所有分片上传任务
list_uploads_result = bucket.list_multipart_upload("test_create_multipart_upload.txt")

# 完成分片上传
parts = [CompletedPart(etag=etag, part_number=1)]
completed = CompletedMultipartUpload(parts=parts)
complete_result = bucket.complete_multipart_upload(
    "test_create_multipart_upload.txt",
    upload_id,
    completed
)

# 取消分片上传
bucket.abort_multipart_upload("test_create_multipart_upload.txt", upload_id)
```