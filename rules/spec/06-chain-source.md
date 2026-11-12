# 06. Chain source

dsverify reads the chain from your own Dogecoin Core, block by block, and checks every block hash against Core before it applies a rule.
The Doge Soft index reads the same blocks from Dogecoin Core with full history, so a replay with the open rules lands on the same state.
