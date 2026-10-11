# 内容包规范 v1

[English](CONTENT_PACKS.md)

**本规范自 v0.7.1 起提供。**内容包格式为 **1**、世界格式为 **2**、联机协议为 **3**。
客户端和服务端需要使用同一构建。旧版使用协议 2，无法加入协议 3 的房间。

官方资源从 `assets/packs/yapshire/pack.json` 加载，社区内容使用相同的加载器、校验和文件格式。
一个世界使用一个完整内容包。本版包含有限地图、图集、物件和已有玩法的交互定义；
依赖解析、脚本 Mod、自动下载美术资源、无限地图和挖掘建造系统留待后续。

## 做出自己的内容包

1. 把程序旁的 `assets/packs/yapshire` 复制为 `assets/packs/my_pack`。
2. 修改副本 `pack.json` 的 `id` 为 `my_pack`，填写自己的 `version`，删除 `legacy_tiles`。
   原有资源 ID 可以保留；新增资源使用自己的命名空间，例如 `my_pack:brick/wall`。
3. 用 [Tiled](https://www.mapeditor.org/) 打开 `maps/town.tmj`。使用有限正交地图、16×16 网格、未压缩 JSON 数组。
4. 执行 `yapshire-server --maps assets/packs/my_pack --check` 校验。
5. Linux/macOS：`YAPSHIRE_PACK=my_pack ./yapshire`。
   PowerShell：`$env:YAPSHIRE_PACK='my_pack'; .\yapshire.exe`。
6. 把完整目录分享给朋友。大家安装并选择同一内容包后，加入房间时自动同步服务端的地图布局。

游戏内编辑器通过“下一张地图”浏览清单中的地图，图层支持翻页，调色板读取全部图集。
画笔和填充会自动连接地形；“物件印章 / P”能一次放下整栋建筑或完整家具，点击位置是印章左上角。
先选择需要绘制的 tile layer。对象层和背景层会保留并显示预览；新增或修改这些对象请在 Tiled 中操作。
金色辅助标记直接读取地图对象的位置。

官方本地覆盖文件仍保存在 Yapshire 数据目录的 `maps/` 中；其他内容包使用 `packs/<包名>/`。
`YAPSHIRE_MAP_DIR` 可覆盖保存目录。保存时会补齐缺失的清单、图集和图片，并按清单路径存放地图，
例如保存目录下的 `maps/town.tmj`。已经存在且内容不同的美术文件不会被覆盖。
尚未生成的其他地图会用上次保存的布局补齐，使目录成为完整内容包；其他地图的未保存草稿仍留在编辑器中。
更新内容包前先关闭游戏，保留自己的地图草稿，并同步更新配套资源。

## 目录与稳定 ID

```text
my_pack/
  pack.json
  terrain/ground.tsj + ground.png
  terrain/water.tsj  + water.png
  buildings/tackle.tsj + tackle.png
  objects/harbor.tsj + harbor.png
  backgrounds/street.png
  maps/town.tmj
  maps/tackle-shop.tmj
```

可直接参考[官方清单](../assets/packs/yapshire/pack.json)：

| 字段 | 用途 |
| --- | --- |
| `format` / `id` / `version` / `tile_size` | 格式版本 `1`、包名、内容版本、网格 `16` |
| `tilesets` | 按顺序登记的外部 `.tsj` 路径 |
| `images` | 允许背景图层引用的 PNG 路径 |
| `maps` | 每张地图的稳定 `id`、相对 `path`、显示 `title` 和 `indoors` |
| `entry` | 初始地图 `map` 及出生点名称 `spawn` |
| `prefabs` | 可重复使用的完整物件图块排列 |
| `terrains` | 按连接掩码排列的 16 个地形变体 |

清单中的路径相对内容包根目录，Tiled 引用相对当前地图或图集，例如 `../terrain/ground.tsj`。
禁止绝对路径、反斜杠、盘符以及指向包外的符号链接。
ID 使用 `namespace:name`，区分大小写且不能重复。命名空间使用小写 ASCII 字母、数字和 `_`，
名称额外允许 `-` 和 `/`。

每个 `.tsj` 图块必须带字符串属性 `id`。图集中的数字 `id` 与地图里的 `firstgid` 只是 Tiled 地址，
不承担资源身份。地图保存字符串属性 `yapshire:palette`，记录上次保存时的稳定 ID 对照表。
重新排列图集时，已有格子通过这个对照表找到原来的资源；翻转信息也会保留。
**不要删除该属性。调整图块编号后，先在 Yapshire 中加载并保存相关地图，再继续用 Tiled 编辑**，
以便更新编号和对照表。没有对照表的新 Tiled 地图按当前图集解释。
删除或重命名仍被引用的 ID 会明确报错；不要把旧 ID 静默挪作其他用途。

图块固定 16×16，图集没有间距和边距，PNG 尺寸是 16 的倍数，单边最多 4096 像素。
动画沿用 Tiled 的 `animation`，要求连续帧及一致的正数帧时长。官方水体包含 27 组四帧动画。

## 坐标、遮挡和碰撞

Tiled 左上角为原点，x 向右、y 向下。角色位置指脚底，换算关系为：
`world_x = tiled_x`，`world_y = 地图行数 × 16 - 64 - tiled_y`。
默认 17 行地图中，Tiled 的 `y=208` 就是原来的地面。地图尺寸和图层数可以改变。

| 定义 | 效果 |
| --- | --- |
| 图块字符串属性 `collision` | `none` 无碰撞、`solid` 实心、`platform` 单向平台 |
| tile layer 布尔属性 `collision` | 设为 `true` 后，该层使用图块的碰撞定义 |
| `solid` 矩形对象 | 独立于美术的实心碰撞区域 |
| 图层数值属性 `z` | 遮挡深度，默认 `-30 + 图层序号 × 6` |
| 图层 `visible` / `opacity` | 控制显示；隐藏美术不会删除物理碰撞 |

角色能站在不同高度的地面上，从下方穿过单向平台，在下落时落到平台上，并受到墙面和天花板阻挡。
掉出地图底部后回到该地图的出生点。镜头范围随地图尺寸变化。
角色深度为 `z=5`；前景物件放在更高的图层，背景家具放在更低的图层。
美术外轮廓与实际占地不一致时，另画 `solid` 矩形。本版没有斜坡、旋转碰撞和可破坏地形。

背景 image layer 使用清单登记的 PNG 和像素偏移。原来的街道插画也变成了内容包里的普通背景层。
天空、云等室外氛围仍是游戏共用表现，作者可用背景层覆盖。
暂不支持嵌套图层、无限分块、压缩数据、带 `gid` 的 tile object、Tiled 模板、多边形和视差层；会明确拒绝这些格式。

## 出生点、门、商店和钓鱼区域

在 `objectgroup` 中设置对象的 **Type**（也兼容旧版导出的 `class`），旋转为零，数字对象 ID 为正数且唯一。
每张地图至少有一个具名出生点。

| Type | 形状 | 属性及行为 |
| --- | --- | --- |
| `spawn` | 点 | 唯一 `name`，供入口和传送门引用 |
| `portal` | 矩形 | 字符串 `target_map`、`target_spawn`；区域内按 E 传送 |
| `shop` | 矩形 | 按 E 打开现有渔具店；店员位置随区域移动 |
| `fishing` | 矩形 | 按 E 开始现有钓鱼小游戏，向右抛竿 |
| `solid` | 矩形 | 实心碰撞，不自动提供美术 |
| `prefab` | 点 | 字符串 `prefab` 指向清单内的物件 ID |

物件包含以格子计的 `width` / `height`、以像素计的 `anchor: [x,y]`，以及按行排列的稳定 ID 数组 `tiles`。
空字符串代表透明格。对象位置减去锚点后必须对齐 16px 网格，完整物件必须位于地图范围内。
这类点对象在 Yapshire 中渲染为完整物件，在 Tiled 中显示为标记；若必须在 Tiled 中看到美术，
使用编辑器的图块印章。物件碰撞另用 `solid` 矩形定义。
交互区域复用现有渔具商店与钓鱼规则，本版不包含自定义商品经济和鱼种表。

增加新地点：复制一张 `.tmj`，在 `pack.json.maps` 中登记新的稳定 ID，放置 `arrival` 出生点，
把门的 `target_map` / `target_spawn` 指向它，再按相同方法设置回程门。
新地图、移动门店入口或移动垂钓区都不需要修改 Rust；使用新图片时先登记相关资源。

## 自动连接地形

官方地面图集包含完整的 **Quay** 十六种边连接变体，可直接用 Tiled 的 Terrain Brush 绘制。
游戏内画笔、填充和擦除也会重新计算该层的相邻地形。

`terrains["my_pack:stone"]` 按掩码 `0..15` 登记稳定 ID，位为：**北 1、东 2、南 4、西 8**。
对应方向连接同种地形时置位。每块地形图块带同名字符串 `terrain` 属性。
全部变体必须位于同一图集；补充对应 `wangsets` 后即可用 Tiled 的地形笔刷。
本版处理单种材质的四边连接，不自动混合两种材质，也不处理角连接地形。

## 联机、旧地图迁移和边界

服务端在开放端口前校验内容包，加入时发送定义、地图布局及世界版本。
客户端比较包 ID、版本、JSON 定义和图片 SHA-256，完全一致后才发送 `world_ready`。
不匹配时显示缺失或不同的资源；PNG 与脚本不会从服务端自动下载。
远程布局仅保存在内存，离开房间恢复本地地图。移动消息携带地图稳定 ID，身处不同地点的玩家互相隐藏；
服务端拒绝未知地图，并按地图边界限制坐标。

原来引用 `harbor.tsj` 的 `town.tmj` / `tackle-shop.tmj` 可以从旧编辑器或服务端目录导入。
若旧目录包含图集和 PNG，必须与原版一致。迁移表覆盖原来的 359 个图块，保留翻转及美术布局，
并补入原有地面碰撞和交互对象。加载不会修改文件；明确点击保存后才写入新目录结构，
保留原字节的 `.tmj.bak`，旧文件仍留在原处。

边界：每包最多 32 张地图、32 个图集、8192 个图块；每图最多 32 层，2–256 列、4–128 行，
每对象层最多 256 个对象；单图 1 MiB，世界消息 8 MiB，单资源文件 4 MiB，加载资源总量 32 MiB。

格式依据 Tiled 的 [JSON 文件规范](https://doc.mapeditor.org/en/stable/reference/json-map-format/)、
[GID 规则](https://doc.mapeditor.org/en/stable/reference/global-tile-ids/)
和[地形规则](https://doc.mapeditor.org/en/stable/manual/terrain/)。
