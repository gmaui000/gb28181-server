## gbserver 信令服务实现：
1. 设备注册
2. 设备心跳
3. 状态信息（在线/离线）4. 设备信息查询
5. 设备代理通道目录信息查询
6. 实现点播；
7. 自动关闭流：流注册超时、无人观看、响应超时等
8. 支持与webzlm多节点部署通信

## TODO:
1. 历史回放
    倍数播放
    拖动播放
2. 云台控制
    转向
    焦距调整
3. 事件配置
4. 手动抓拍、自动抓拍、定时抓拍
5. 图片上传
6. 视频下载
7. 级联
8. 统一响应码
9. 按需推流
10. 图片AI识别-插件化业务场景
11. 多数据库配置

## 测试方法：

curl -X POST "http://localhost:6070/live/play" -H "Content-Type: application/json" -d '{ "gb_code": "34020000001180000000", "setup_type": "passive","channel_id": "34020000001320000001"}'


ffplay -i rtsp://localhost:8554/rtp/000015C9?token_id=cowa_test