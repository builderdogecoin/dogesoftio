# 05. Things that are not inscriptions or names

- Broken `cdrc-20` entries are not inscriptions.
- `.dogemap` tiles are not names. A tile is a plain-text body such as `1234.dogemap`, and a plain-text claim
  with the `.dogemap` ending is never read as a name. A JSON `p:dns` claim is only normalized (one dot with
  text on both sides), so its name is read whatever its label or ending, `.dogemap` included. The label and
  ending rules for names apply to plain-text claims only.

Test vectors: `vectors/inscriptions/`, `vectors/names/`.
