# Meeting notes — model evaluation session

Date: 2026-03-15
Attendees: team

## API keys used during session

We tested the eval pipeline using:
- Anthropic key: sk-ant-api03-FAKEKEYFORTESTING1234567890abcdefghijklmnopqrstuvwxyz1234567890ab-AAAAAAA
- OpenAI key: sk-proj-FAKEOPENAIKEY1234567890abcdefghijklmnopqrstuvwxyz1234567890AAAAAAAAAAAAA
- Internal token: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.FAKE.FAKE

## Discussion

Ran Mixtral 8x7B against Switch Transformer on the standard eval suite.
Mixtral scored higher on reasoning tasks. Switch was faster per token.

## Action items

- Archive the API keys above after the session (they are already rotated)
- Write up findings in the research wiki
- Contact jane@example.com for access to the private eval dataset

## Raw scores

| Model | MMLU | HellaSwag | ARC |
|-------|------|-----------|-----|
| Mixtral 8x7B | 70.6 | 81.2 | 66.0 |
| Switch-XXL | 68.3 | 79.5 | 63.1 |
