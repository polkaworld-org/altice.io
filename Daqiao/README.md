# DaQiao

[中文说明](#中文说明-1)
-----

## 1 Background and Goals

Polkadot is a heterogeneous multi-chain architecture, which aims to be an extensible heterogeneous multi-chain framework. Under the basic function of ensuring security and transmission, through the incentive mechanism of untrusted nodes, it weakens the endogenous binding relationship and realizes the interconnection and interoperability between different blockchain systems.

Although blockchain have experienced 10 years of development, the existing public chain infrastructure is still immature, and the mainstream public blockchain are completely open, which is not suitable for the application scenario of commercial companies. Therefore, many practical blockchain business landed in the form of alliance blockchain. At present, there are many valuable data in various consortium blockchain, forming different data islands.

1. Open the cross-chain communication between the existing alliance chain and the mainstream open chain;

2. Ensuring data privacy between different chains, ensuring data availability and invisibility, thus protecting the interests of the alliance itself;

Based on the above background, our Altice. IO team's contest content is mainly through substrate to get through **Fabric** and **Eth**. Subsequently, we will continue to implement cross-chain privacy protection, and promote the existing alliance chain ecosystem access to Polkadot ecosystem, and ultimately build the bottom of the Internet's credit value.

## 2 Integral Design

![图片](https://github.com/altice-io/Daqiao/blob/master/image/Daqiao.png?raw=true)

## 3 Detailed Design

The whole architecture consists of three modules: runtime, endorser and qrml. Among them, runtime is the main logic of chain operation, endorser is the main logic of bridge, and qrml is the dependent runtime lib. In DaQiao, it mainly relies on a token protocol of erc20. The detailed design of each module is shown below.

### 3.1 Runtime
#### Structure

` PledgeInfo `is the details of a pledge, which is used to describe the basic information of a pledge.
```
// PledgeInfo Describe the details of the pledge record.
pub struct PledgeInfo <U>{
  // ChainId of the third party chain
  chain_id: ChainId,
  // Txid in the third party chain
  ext_txid: ExtTxID,
  // Account of DaQiao
  account_id: U,
  // The amount of money pledged
  pledge_amount: u128,
  // Can be withdraw
  can_withdraw: bool,
  // The withdraw history of pledge history
  withdraw_history: Vec<ExtTxID>
}
```

#### Storage

Runtime mainly stores `ChainToken`and `PledgeRecords`, and `ChainToken` stores the external chain information registered with DaQiao. ` PledgeRecords `stores the details of all pledges.

```
    // Third party chain Id => TokenId
    ChainToken get(chain_token): map u32 => Option<T::TokenId>;

    // Third party chain TxId => PledgeInfo
    PledgeRecords get(pledge_records): map Vec<u8> => PledgeInfo<T::AccountId, T::TokenBalance>;

```

### Interface

Runtime has three main interfaces, namely `register`, `pledge`, `withdraw`.

**register** is the interface between the external chain and DaQiao.

```
    pub fn register(origin, chain_id: u32, token_id: T::TokenId) -> Result {
      Self::_register(origin, chain_id, token_id)
    }
    
```

**pledge** External Chain Trading Interface to DaQiao Pledge.

```
    pub fn pledge(origin, chain_id: u32, ext_txid: Vec<u8>, amount: T::TokenBalance, reciever: T::AccountId) -> Result {
      Self::_pledge(origin, chain_id, ext_txid, amount, reciever)
    }
```
The main process of pledge is as follows:
![图片](https://github.com/altice-io/Daqiao/blob/master/image/pledge.png?raw=true)

**withdraw** is the interface to revoke the pledge of external chain.

```
    pub fn withdraw(origin, chain_id: u32, ext_txid: Vec<u8>, ext_address: Vec<u8>) -> Result {
      Self::_withdraw(origin, chain_id, ext_txid, ext_address)
    }

```

The main process of revocation is as follows:
![图片](https://github.com/altice-io/Daqiao/blob/master/image/withdraw.png?raw=true)

## 4 RoadMap

1. Oct. ~ Nov. in 2019: Perfecting Basic Functions；
2. Nov. ~ Dec. in 2019:Support Eth and Fabric exchange within DaQiao；
3. Dec.~ Jan. in 2020: Support Wallet；

=====
# 中文说明

## 1 背景和目标

Polkadot 是一种异构的多链架构，旨在成为可扩展的异构多链框架，在确保安全和传输的基本功能下，通过非信任节点的激励机制，弱化内生绑定关系，实现不同区块链系统之间的互联互通。

区块链虽然已经经历过了10年的发展，但是现有的公链基础设施仍然不成熟，而且主流的公链都是完全开放的，不适合商业公司的应用场景。所以很多实际的区块链业务落地采取的是联盟链的形式。目前已经有很多很有价值的数据在各个联盟内部，形成了不同链的数据孤岛。 

我们altice.io战队的愿景是通过区块链构建互联网时代的信用价值底层。未来的信用价值底层的形式一定是通过隐私跨链的方式实现不同联盟链与公开链之间数据的跨链互通。实现这样的愿景主要有两个难点：

 1. 一是打通现有联盟链与主流公开链之间的跨链互通；
 2. 二是保障不同链之间的数据隐私，确保数据可用不可见，从而保障联盟自身的利益；

基于以上背景，我们Altice.io战队的参赛内容主要是通过substrate打通Fabric和Eth。后续会继续基于此实现跨链间的隐私保护，并推动既有的联盟链生态接入Polkadot生态，最终构建起互联网的信用价值底层。

## 2 整体设计

![图片](https://github.com/altice-io/Daqiao/blob/master/image/Daqiao.png?raw=true)

## 3 详细设计

整个架构主要包括3个模块，分别是runtime、endorser和qrml。其中runtime是链操作的主要逻辑，endorser是桥的主要逻辑，qrml是依赖的runtime lib。在DaQiao中，主要依赖一个erc20的token协议。各模块的详细设计如下所示。

### 3.1 Runtime

#### Structure

`PledgeInfo` 是质押详情，用于描述一次质押的基本信息。
```
// PledgeInfo Describe the details of the pledge record.
pub struct PledgeInfo <U>{
  // ChainId of the third party chain
  chain_id: ChainId,
  // Txid in the third party chain
  ext_txid: ExtTxID,
  // Account of DaQiao
  account_id: U,
  // The amount of money pledged
  pledge_amount: u128,
  // Can be withdraw
  can_withdraw: bool,
  // The withdraw history of pledge history
  withdraw_history: Vec<ExtTxID>
}
```

#### Storage
Runtime主要存储了 `ChainToken` 和`PledgeRecords` 两类信息， `ChainToken` 存储了注册到DaQiao的外链信息。`PledgeRecords` 存储了所有质押的详情。
```
    // Third party chain Id => TokenId
    ChainToken get(chain_token): map ChainId => Option<TokenId<T>>;
    
    // Third party chain TxId => PledgeInfo
    PledgeRecords get(pledge_records): map ExtTxID => PledgeInfo<T::AccountId>;
```
### Interface
Runtime主要有3个接口，分别是`register`,`pledge`,`withdraw`。
**register** 是外链和DaQiao关联的接口。
```
    pub fn register(origin, chain_id: u32, token_id: T::TokenId) -> Result {
      Self::_register(origin, chain_id, token_id)
    }
    
```
**pledge** 外链交易到DaQiao质押的接口。
```
    pub fn pledge(origin, chain_id: u32, ext_txid: Vec<u8>, amount: T::TokenBalance, reciever: T::AccountId) -> Result {
      Self::_pledge(origin, chain_id, ext_txid, amount, reciever)
    }
```
质押的主要流程如下所示：
![图片](https://github.com/altice-io/Daqiao/blob/master/image/pledge.png?raw=true)

**withdraw**是撤销外链质押的接口。
```
    pub fn withdraw(origin, chain_id: u32, ext_txid: Vec<u8>, ext_address: Vec<u8>) -> Result {
      Self::_withdraw(origin, chain_id, ext_txid, ext_address)
    }

```
撤销的主要流程如下所示：
![图片](https://github.com/altice-io/Daqiao/blob/master/image/withdraw.png?raw=true)

## 4 RoadMap

1. Oct. ~ Nov. in 2019: 完善项目基础功能；
2. Nov. ~ Dec. in 2019: 支持Eth和Fabric在DaQiao内兑换；
3. Dec.~ Jan. in 2020: 支持钱包等周边工具；
