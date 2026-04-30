"""生成6个不同的32x32像素风格Agent图标"""
from PIL import Image, ImageDraw

def create_default_icon():
    """默认 - 蓝色小人"""
    img = Image.new('RGBA', (32, 32), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    # 身体
    draw.rectangle([12, 10, 20, 24], fill=(100, 180, 255))
    # 头
    draw.rectangle([13, 4, 19, 11], fill=(140, 210, 255))
    # 眼睛
    draw.rectangle([14, 6, 15, 7], fill=(255, 255, 255))
    draw.rectangle([17, 6, 18, 7], fill=(255, 255, 255))
    # 腿
    draw.rectangle([13, 24, 14, 28], fill=(80, 150, 230))
    draw.rectangle([17, 24, 18, 28], fill=(80, 150, 230))
    return img

def create_wizard_icon():
    """法师 - 紫色法杖"""
    img = Image.new('RGBA', (32, 32), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    # 身体 (紫色袍子)
    draw.rectangle([11, 12, 21, 24], fill=(140, 80, 200))
    # 头
    draw.rectangle([13, 5, 19, 12], fill=(160, 110, 220))
    # 帽子
    draw.polygon([(16, 0), (10, 5), (22, 5)], fill=(120, 60, 180))
    # 眼睛
    draw.rectangle([14, 7, 15, 8], fill=(255, 200, 50))
    draw.rectangle([17, 7, 18, 8], fill=(255, 200, 50))
    # 法杖
    draw.rectangle([22, 4, 23, 26], fill=(180, 140, 80))
    draw.rectangle([21, 2, 24, 5], fill=(200, 100, 255))
    return img

def create_fox_icon():
    """狐狸 - 橙色"""
    img = Image.new('RGBA', (32, 32), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    # 身体
    draw.rectangle([10, 12, 22, 22], fill=(220, 120, 50))
    # 头
    draw.rectangle([11, 6, 21, 13], fill=(230, 140, 60))
    # 耳朵
    draw.polygon([(11, 6), (9, 1), (14, 5)], fill=(200, 100, 40))
    draw.polygon([(21, 6), (23, 1), (18, 5)], fill=(200, 100, 40))
    # 眼睛
    draw.rectangle([13, 8, 14, 9], fill=(30, 30, 30))
    draw.rectangle([18, 8, 19, 9], fill=(30, 30, 30))
    # 尾巴
    draw.rectangle([22, 14, 26, 18], fill=(200, 100, 40))
    draw.rectangle([25, 12, 27, 15], fill=(240, 200, 150))
    return img

def create_dragon_icon():
    """龙 - 绿色"""
    img = Image.new('RGBA', (32, 32), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    # 身体
    draw.rectangle([10, 10, 22, 22], fill=(60, 160, 60))
    # 头
    draw.rectangle([11, 4, 21, 11], fill=(70, 180, 70))
    # 角
    draw.rectangle([12, 1, 13, 5], fill=(100, 80, 60))
    draw.rectangle([19, 1, 20, 5], fill=(100, 80, 60))
    # 眼睛
    draw.rectangle([13, 6, 15, 8], fill=(255, 50, 50))
    draw.rectangle([17, 6, 19, 8], fill=(255, 50, 50))
    # 翅膀
    draw.polygon([(10, 10), (4, 6), (4, 16)], fill=(50, 140, 50))
    draw.polygon([(22, 10), (28, 6), (28, 16)], fill=(50, 140, 50))
    # 尾巴
    draw.rectangle([22, 18, 26, 20], fill=(50, 140, 50))
    draw.rectangle([25, 16, 27, 18], fill=(200, 80, 80))
    return img

def create_lion_icon():
    """狮子 - 金色"""
    img = Image.new('RGBA', (32, 32), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    # 鬃毛 (圆形)
    draw.ellipse([6, 2, 26, 18], fill=(220, 160, 40))
    # 脸
    draw.rectangle([10, 5, 22, 15], fill=(240, 200, 100))
    # 眼睛
    draw.rectangle([12, 7, 14, 9], fill=(40, 40, 40))
    draw.rectangle([18, 7, 20, 9], fill=(40, 40, 40))
    # 鼻子
    draw.rectangle([15, 10, 17, 12], fill=(180, 100, 60))
    # 身体
    draw.rectangle([11, 15, 21, 26], fill=(220, 160, 40))
    return img

def create_robot_icon():
    """机器人 - 灰色金属"""
    img = Image.new('RGBA', (32, 32), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    # 身体
    draw.rectangle([10, 14, 22, 24], fill=(120, 130, 140))
    # 头
    draw.rectangle([11, 5, 21, 14], fill=(140, 150, 160))
    # 天线
    draw.rectangle([15, 1, 17, 5], fill=(100, 110, 120))
    draw.rectangle([14, 0, 18, 2], fill=(255, 100, 100))
    # 眼睛 (LED)
    draw.rectangle([12, 7, 14, 9], fill=(0, 255, 0))
    draw.rectangle([18, 7, 20, 9], fill=(0, 255, 0))
    # 手臂
    draw.rectangle([7, 15, 10, 22], fill=(100, 110, 120))
    draw.rectangle([22, 15, 25, 22], fill=(100, 110, 120))
    return img

# 生成所有图标
icons = {
    'default': create_default_icon(),
    'wizard': create_wizard_icon(),
    'fox': create_fox_icon(),
    'dragon': create_dragon_icon(),
    'lion': create_lion_icon(),
    'robot': create_robot_icon(),
}

output_dir = 'client/assets/textures/agents'
for name, icon in icons.items():
    path = f'{output_dir}/{name}.png'
    icon.save(path, 'PNG')
    print(f'保存: {path}')

print('所有图标生成完成!')
