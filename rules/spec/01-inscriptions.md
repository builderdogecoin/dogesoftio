# 01. Inscriptions

## Envelope from input 0 only {#envelope}

An inscription envelope is read from the scriptSig of **input 0** of the reveal transaction. Envelope data in any other input does not create an inscription.

## Reinscription creates nothing {#reinscription}

If the koinu at offset 0 of the reveal's inputs already carries an inscription, the envelope does not create a new inscription.

Test vectors: `vectors/inscriptions/`.
