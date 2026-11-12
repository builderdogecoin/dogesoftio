# The Doge Soft Standard (`ds`)

One small tag that says what a transaction **is**: a sale, a launch, a bridge deposit, a governance action. Full spec: [docs.dogesoft.io/inscription-standard](https://docs.dogesoft.io/inscription-standard).

## The whole tag

```json
{"ds":{"action":"<VERB>","ref":"<Doge Soft-issued id>"}}
```

- `action` is an **UPPERCASE** verb.
- `ref` names the signed record. It is optional where the verb doesn't need one.
- Nothing else sits inside `ds`.

Example on a DRC-20 transfer sold on the marketplace:

```json
{"p":"drc-20","op":"transfer","tick":"DU$T","amt":"1000","ds":{"action":"BUY","ref":"dogesoft.io/r_…"}}
```

## Ref format

`ref` is `<domain>/<local>`, with `dogesoft.io` as the default domain. Locals keep their kinds: `r_<n>` for a sale record, `lp_<slug>` for a collection record.

The record is signed with a standard Dogecoin `signmessage` over the text `Doge Soft`, a newline, then the JSON of the record. Any Dogecoin wallet can check that signature.

## Where it rides

For marketplace sales, the tag is an `OP_RETURN` note inside the sale transaction itself: no extra transaction. A governance action is the inscription itself.

## Verbs

| Area | Verbs |
|---|---|
| Marketplace | `BUY` `AUCTION_SOLD` `OFFER_ACCEPT` `GIFT` `BUNDLE` `PARTIAL_FILL` `COLLECTION_TRANSFER` |
| Launchpad | `LAUNCH_DEPLOY` `LAUNCH_MINT` |
| Social | `TIP` `REVEAL` `BOOST` |
| Bridge | `BRIDGE_L1_DEPOSIT` and related bridge verbs |
| Governance | `GOV_CREATE` `GOV_UPDATE` `GOV_PROPOSE` `GOV_VOTE` `GOV_FINALIZE` `GOV_EXECUTE` `GOV_CANCEL` `GOV_DELEGATE` `GOV_APPOINT` `GOV_REMOVE` `GOV_RESIGN` and more |

Anyone can implement the standard. The spec text is CC-BY-4.0: share it, build on it, credit Doge Soft.
