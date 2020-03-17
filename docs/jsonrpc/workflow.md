# Typical workflows

<!---
Reference for mermaid
https://mermaid-js.github.io/mermaid/#/sequenceDiagram
-->


## Address Generation

<mermaid>
sequenceDiagram
    rect rgb(0, 255, 0, .1)
        User->>+Exchange: Deposit Request
        Exchange-->>+Service: key_generate()
        Service-->>-Exchange: mnemonic
        Exchange-->>+Service: key_derive(mnemonic, path)
        Service-->>-Exchange: public_key, private_key address
        Exchange->>+User: Address
    end
    % This is not covered
    Note over Exchange,Node: This is not covered
    Exchange->>+Node: Monitor Address
    Node-->>-Exchange: change event
    Exchange->>+Node: Get_balance(address)
    Node-->>-Exchange: balance  
</mermaid>

## Signing

<mermaid>
sequenceDiagram
    Note over Exchange,Node: COMPLETE
</mermaid>

::: danger Old diagrams
Work in progress
:::

```asciidoc
    /-----------/           /--------------/        /---------/
    /   USER    /           /   EXCHANGE   /        /  NODE   /
    /-----------/           /--------------/        /---------/
          |                        |                     |
          |------Sell Request----->|                     |
          |                        | key_derive()        |
          |<-----Address-----------|                     |
          |                        |                     |
          |------------------Send Transaction----------->|
          |                        |                     |
          |                        |<-----Notify---------|
          |                        |                     |
          |<------Success----------|                     |

    /-----------/           /--------------/        /---------/
    /   USER    /           /   EXCHANGE   /        /  NODE   /
    /-----------/           /--------------/        /---------/
          |                        |                     |
          |------Buy (address)---->|                     |
          |                        | key_derive()        |
          |                        | tx_create()         |
          |                        | sign_transaction()  |
          |                        |                     |
          |                        |-----Sentd Tx ------>|
          |                        |                     |
          |                        |                     |
          |                        |<-----Notify---------|
          |                        |                     |
          |<------Success----------|                     |

```