import express from 'express';
import { ApiPromise, WsProvider } from '@polkadot/api';
import { u32 } from '@polkadot/types';
import { isMainThread } from 'worker_threads';
import { Keyring } from '@polkadot/keyring';
import { Text } from "@polkadot/types";
import { RSA_NO_PADDING } from 'constants';
import child_process from "child_process";
const Web3 = require('web3');
const fs = require("fs");

const CHAINID_FABRIC = "1"
const CHAINID_ETH = "2"

const chains = {
    [CHAINID_ETH]: {
        transfer: eth_transfer,
        query_tx: eth_query_tx,
        bank_address: "",
    },
    [CHAINID_FABRIC]: {
        transfer: fabric_transfer,
        query_tx: fabric_query_tx,
        bank_address: "bc347b901e7da41e726a7d9dd790fa4e81274822bb9ac006e5a822751315f701",
    }
}

function initApi() {
    const wsProvider = new WsProvider('ws://127.0.0.1:9944');
    return ApiPromise.create({
        provider: wsProvider,
        types: {
            ChainId: 'u32',
            Erc20MintableBurnable: {},
            ExtTxID: 'Vec<u8>',
            PledgeInfo: {
                chain_id: "u32",
                ext_txid: "Vec<u8>",
                account_id: "AccountId",
                pledge_amount: "TokenBalance",
                can_withdraw: "bool",
                withdraw_history: "Vec<Vec<u8>>",
                withdraw_address: "Vec<u8>",
            },
            Symbol: 'Vect<u8>',
            TokenBalance: 'u128',
            TokenDesc: 'Vec<u8>',
            TokenId: 'u32',
        }
    });
}


var alice;
var api;
var keyring;
var web3 = new Web3(new Web3.providers.HttpProvider('http://127.0.0.1:8545'));
var private_key = fs.readFileSync("./pk", "utf8");

var app = express();
app.use(express.json());
app.use(express.urlencoded({ extended: true }));

async function main() {
    api = await initApi();
    keyring = new Keyring({ type: 'sr25519' });
    alice = keyring.addFromUri('//Alice');
    var bob = keyring.addFromUri('//Bob');

    app.use(function (err, req, res, next) {
        res.status(500).send(err);
    })

    app.get('/', async function (req, res) {
        res.send("hello world");
    });

    /*
    {
        "txid":"",
        "chainid":"",
    }
    */
    app.post("/pledge", async (req, res) => {
        try {
            const bank_addr = "";
            var body = req.body;
            var chainid = body["chainid"];
            var txid = body["txid"];

            const chain = chains[chainid];
            if (!chain) {
                res.status(400).send("bad chainid");
                return
            }

            var tx = chain.query_tx(txid);
            if (tx.to != chain.bank_address) {
                res.status(400).send("bad to address, not bank");
                return
            }
            if (await daqiao_pledge_exists(txid)) {
                res.status(400).send("already pledged");
                return;
            }
            await daqiao_pledge(chainid, txid, tx.address, tx.amount);

            res.send(new Text(txid).toHex());
        } catch (e) {
            res.status(500).send(e.stack);
        }
    });

    /*
    {
        "txid":"",
        "chainid":"",
    }
    */
    app.post("/withdraw", async (req, res) => {
        try {
            const bank_addr = "";
            var body = req.body;
            var txid = body["txid"];
            var chainid = body["chainid"];
            const chain = chains[chainid]
            if (!chain) {
                res.status(400).send("bad chainid");
                return
            }

            var pledge_info = await daqiao_query_pledge_info(txid);
            if (pledge_info == null) {
                res.status(400).send("can't withdraw");
                return;
            }
            await chain.transfer(pledge_info.to, pledge_info.amount)
            res.send("ok");
        } catch (e) {
            res.status(500).send(e.stack);
        }
    });

    app.listen(3000, function () {
        console.log('Example app listening on port 3000!');
    });
}

function fabric_query_tx(txid) {
    var response = child_process.spawnSync("./fbtool", ["query", "--txid", txid]);
    if (response.status != 0) {
        return null
    }
    var args = JSON.parse(response.stdout);
    // transfer alice 10 0x123456
    return {
        to: args[1],
        amount: args[2],
        address: args[3],
    }
}

function fabric_transfer(to, amount) {
    var response = child_process.spawnSync("./fbtool", ["chaincode", "invoke", "transfer", to, amount]);
    if (response.status != 0) {
        console.log(response.stdout.toString());
        console.log(response.stderr.toString());
        throw ("bad txid");
    }
}
/*
@return 
{
    to:"",
    amount:"",
    address:"",
}
*/
function eth_query_tx(txid) {
    var tx = web3.eth.getTransaction(txid);
    var to = tx['to'];
    var amount = tx['value'];
    var address = tx['input'];
    console.log(to, amount, address);
    return {
        to: to,
        amount: amount,
        address: address,
    };
}

function eth_transfer(to, amount) {
    // var privateKey = new Buffer('e331b6d69882b4cb4ea581d88e0b604039a3de5967688d3dcffdd2270c0fd109', 'hex');
    var privateKey = new Buffer(private_key, 'hex');
    var rawTx = {
    // nonce: '0x00',
    gasPrice: '0x09184e72a000',
    gasLimit: '0x2710',
    to: to, //'0x0000000000000000000000000000000000000000',
    value: amount, //'0x00',
    };

    var tx = new Tx(rawTx);
    tx.sign(privateKey);

    var serializedTx = tx.serialize();
    web3.eth.sendSignedTransaction('0x' + serializedTx.toString('hex')).on('receipt', console.log);
}

async function daqiao_pledge(chainid, txid, to, amount) {
    var tx = api.tx.daqiao.pledge(chainid, txid, amount, to);
    console.log(tx.toString())
    var unsub = await tx.signAndSend(alice, (result) => {
        console.log(`Current status is ${result.status}`);
        if (result.status.isFinalized) {
            console.log(`Transaction included at blockHash ${result.status.asFinalized}`);
            unsub();
        }
    });
}

async function daqiao_withdraw(chainid, txid, to, amount) {
    var unsub = await api.tx.daqiao.pledge(chainid, txid, amount, to).signAndSend(alice, (result) => {
        console.log(`Current status is ${result.status}`);
        if (result.status.isFinalized) {
            console.log(`Transaction included at blockHash ${result.status.asFinalized}`);
            unsub();
        }
    });
}

async function daqiao_query_pledge_info(pledgeid) {
    var response = await api.query.daqiao.pledgeRecords(pledgeid);
    console.log("query_pledge_info ", response.toString());
    if (response.ext_txid.eq("0x")) {
        return null;
    }
    if (response.can_withdraw.isTrue) {
        return null;
    }

    return {
        to: response.withdraw_address.toString(),
        amount: response.pledge_amount.toString(),
    }
}

async function daqiao_pledge_exists(txid) {
    var response = await api.query.daqiao.pledgeRecords(txid);
    return !response.ext_txid.eq("0x");
}

main()
