<div style="display: flex, margin: 8px">
    <img src="./screenshot/1.png" width="100"/>
    <img src="./screenshot/2.png" width="100"/>
    <img src="./screenshot/3.png" width="100"/>
    <img src="./screenshot/4.png" width="100"/>
    <img src="./screenshot/5.png" width="100"/>
    <img src="./screenshot/6.png" width="100"/>
    <img src="./screenshot/7.png" width="100"/>
    <img src="./screenshot/8.png" width="100"/>
</div>

[English Documentation](./README.md)

#### 警告
**如果你没有审查代码的能力，我不建议你使用这个Solana钱包。如果你一定要使用这个钱包，你自己承担风险。**

#### 简介
这个一个基于Rust和Slint-ui的`Solana`钱包。你可以编译到Linux、Windows和Android平台。不过，Android 平台的用户体验并不是太友好。

#### 功能
- [x] 创建和恢复账户
- [x] 生成子账户
- [x] 地址簿
- [x] 交易历史
- [x] 测试模式：支持测试网和开发网
- [x] 在主网发送和接收Sol和Token
- [x] 支持给地址生成QrCode

#### TODO
- [ ] 获取 spl token 价格
- [ ] 在Android平台复制和粘贴文本

##### 安卓平台编译信息
- `min-sdk-version = 23`
- `target-sdk-version = 32`

#### 如何构建?
- 安装 `Rust` 和 `Cargo`
- 安装 Android `sdk`, `ndk`, `jdk17`, 和设置对应的环境变量
- 运行 `make` 编译安卓平台程序
- 运行 `make debug` 调试桌面平台程序
- 运行 `make build-desktop-release` 编译桌面平台程序
- 参考 [Makefile](./Makefile) 了解更多信息

#### 参考
- [Slint Language Documentation](https://slint-ui.com/releases/1.0.0/docs/slint/)
- [github/slint-ui](https://github.com/slint-ui/slint)
- [Viewer for Slint](https://github.com/slint-ui/slint/tree/master/tools/viewer)
- [LSP (Language Server Protocol) Server for Slint](https://github.com/slint-ui/slint/tree/master/tools/lsp)
- [developer.android.com](https://developer.android.com/guide)
- [solana faucet](https://faucet.solana.com/)
- [solana explorer](https://explorer.solana.com/?cluster=testnet)
- [solana fm is a next generation explorer](https://solana.fm/)
- [solana cookbook](https://solanacookbook.com/#contributing)
- [spl token faucet](https://spl-token-faucet.com/?token-name=USDC-Dev)
- [How do we get test stablecoins on Solana like USDC or USDT?](https://solana.stackexchange.com/questions/1513/how-do-we-get-test-stablecoins-on-solana-like-usdc-or-usdt)
- [solana wiki](https://solana.wiki/)
- [solana docs](https://solana.com/docs)
- [examples](https://github.com/solana-developers/program-examples)
- [A Complete Guide to Solana Development for Ethereum Developers](https://solana.com/developers/evm-to-svm/complete-guide)
- [Solana Development Course](https://www.soldev.app/course)
