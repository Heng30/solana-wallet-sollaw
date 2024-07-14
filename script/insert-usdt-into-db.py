import sqlite3
import uuid
import json

db = "/home/blue/.local/share/sollaw/sollaw.db"
conn = sqlite3.connect(db)
cursor = conn.cursor()

tokens = [
    ["USDT-main", "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"],
    ["USDC-main", "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"],
    ["ray-main", "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R"],
]

for token in tokens:
    data_uuid = str(uuid.uuid4())
    data = {
        "uuid": data_uuid,
        "network": "test",
        "symbol": token[0],
        "icon_extension": "",
        "account_address": "GEScvfEF1Xt2oyrnJij5V5DYSmPjuUt45DfUs3VFrsED",
        "mint_address": token[1],
        "balance": "0",
        "balance_usdt": "$0.00",
        "token_account_address": "BbPoqgM2itsDqvXbLKzLJFawFTjwo86HvNd3Rc4bLn6k",
        "decimals": 6,
    }
    json_string = json.dumps(data)
    # print(json_string)

    cursor.execute(
        "INSERT INTO tokens (uuid, data) VALUES (?, ?)", (data_uuid, json_string)
    )
    conn.commit()


conn.close()
