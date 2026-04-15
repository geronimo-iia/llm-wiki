# Step-by-Step Guide

Let’s review how an individual would develop one of these wikis. 

## Step 1: Obtain your resources

You need to accumulate everything – articles that you have saved, books enjoyed, notes you have created, transcripts from discussions, and even your very own historical conversations. All these materials are your raw materials, just as ore must undergo refining before use. 

## Step 2: Classify prior to extracting (Ingest)

One of the best practices from this community is to not treat all documents in the same fashion. 
For example, a 50-page research white paper requires extraction on a section-by-section basis while a tweet or social media thread only requires a primary insight and corresponding context. 
Likewise, a meeting transcript requires extraction of decisions that were made, action items that are to be carried out and key quotations. 
By first classifying the type of document will help extract the right type of information to the correct amount of detail. 


> I just added a new article to raw/articles/
> 2026-04-mixture-of-experts-efficiency.md
> Please ingest it.

```LLM Response:

I've read the article. Here are the key takeaways:

MoE models achieve 4x throughput at the same quality level as dense models
The new routing strategy improves efficiency 23% over Switch Transformer
This contradicts our existing wiki claim in concepts/dense-vs-sparse.md
that states "dense models are always more efficient below 10B parameters"

Classification: Research article (not peer-reviewed whitepaper) — applying
article-level extraction depth (key claims + evidence, not section-by-section).

I will make the following changes:

CREATE wiki/sources/summary-moe-efficiency-2026.md
UPDATE wiki/concepts/mixture-of-experts.md (new routing data)
UPDATE wiki/concepts/scaling-laws.md (new benchmark numbers)
FLAG wiki/concepts/dense-vs-sparse.md (contradiction with point 3)
UPDATE wiki/index.md
APPEND wiki/log.md

Shall I proceed?

```

## Step 3: The AI writes wiki pages (Query)

You will feed your source materials into your AI’s LLM via a structured query. It will allow the LLM to produce one or more wiki pages that conform to the established template of having: a frontmatter block (YAML), a TLDR sentence, the body of the content, and the counterarguments/data gaps.

> Compare the routing strategies in MoE models across our sources.
> Which approach has the best efficiency/quality tradeoff?

```LLM Response:

[Reading wiki/index.md... Found 3 relevant pages]
[Reading wiki/concepts/mixture-of-experts.md...]
[Reading wiki/sources/summary-moe-efficiency-2026.md...]
[Reading wiki/sources/summary-switch-transformer.md...]

MoE Routing Strategies: Synthesis

Based on 3 sources in the wiki:

Strategy	Throughput	Quality Delta	Best For
Top-K	2.1x	Baseline	Simplicity
Expert Choice	3.4x	+2%	Quality priority
Hash Routing	4.0x	-1%	Max throughput

The expert-choice approach ([[mixture-of-experts]]) gives the best quality/efficiency tradeoff for models above 10B parameters. Below 10B, the routing overhead relative to model size makes Top-K preferable.

Note: our wiki currently has a contradiction on this point in [[dense-vs-sparse]] that should be resolved.
```

>> This is great. File it as a wiki page.

``` LLM
Creating wiki/comparisons/moe-routing-strategies.md with type: query-result...
Updating wiki/index.md...
Adding backlinks from wiki/concepts/mixture-of-experts.md...
Appending to wiki/log.md...
Done.
```

## Record your questions

This is one of the most under-appreciated features of the system. 
When you ask the LLM a well-formed question and receive a response that provides valuable insight. 

For example, a comparison between two frameworks, or an explanation of how two concepts are related, you save that response as a new wiki page tagged with the label query-result. As time goes on, your best thinking has been collected rather than lost in chat logs.


```

Activity Log
[2026-04-06] ingest | MoE Efficiency Article

Source: raw/articles/2026-04-mixture-of-experts-efficiency.md
Pages created: wiki/sources/summary-moe-efficiency-2026.md
Pages updated: wiki/concepts/mixture-of-experts.md,
    wiki/concepts/scaling-laws.md
Notes: Contradicts dense-vs-sparse.md claim below 10B params. Flagged.

[2026-04-06] query | MoE Routing Strategy Comparison

Question: Compare routing strategies across sources
Pages read: concepts/mixture-of-experts.md, 3 source summaries
Output filed: wiki/comparisons/moe-routing-strategies.md

[2026-04-05] lint | Weekly Health Check

Contradictions: 2 | Orphans: 3 | Missing pages: 4
Action: Queued RLHF and Constitutional AI pages for next session


```


## Conduct lint passes

At appropriate intervals, you ask the LLM to audit the entire wiki for contradictions or inconsistencies between pages, and to indicate those statements which have been rendered obsolete by a more recent source. 
Additionally, the LLM will provide input on identifying orphan pages (i.e., pages that have no links pointing to them), and for providing a list of concepts that are referenced within the existing content but are not yet represented by their own respective pages.


Sample comming soon
