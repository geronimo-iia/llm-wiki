# Quick note: transformer attention and MoE

The attention mechanism (see [[wiki://notes/concepts/attention-mechanism]]) is
the core operation replaced by MoE feed-forward layers in sparse transformers.

MoE replaces the dense FFN sublayer, not the attention sublayer. Attention
still runs on every token; only the FFN is made sparse.

This is a common source of confusion: people hear "only k/n parameters active"
and assume attention is also sparse. It is not.

Source type: note
