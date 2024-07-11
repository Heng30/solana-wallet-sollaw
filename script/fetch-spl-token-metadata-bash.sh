#! /bin/bash

USDC_MINT_ADDRESS="EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
USDT_MINT_ADDRESS="Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"
HELIUS_FREE_API_KEY="335639a1-e34c-4b26-91bc-83b898c5a948"

DATA='{"jsonrpc":"2.0","id":"text","method":"getAssetBatch","params":{ "ids": ['\"$USDC_MINT_ADDRESS\"','\"$USDT_MINT_ADDRESS\"']}}'

URL='https://mainnet.helius-rpc.com/?api-key='$HELIUS_FREE_API_KEY

# echo $DATA
# echo $URL

curl -L -X POST -H 'Content-Type: application/json' $URL -d "$DATA"
