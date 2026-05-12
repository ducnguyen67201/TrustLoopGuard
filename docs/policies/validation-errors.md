# Policy Validation Errors

Policy validation should return errors that point to fields a user can fix.

## Invalid `id`

```text
id: id must use lowercase letters, numbers, '-' or '_'
```

Use a stable slug:

```yaml
id: refund-guarantee
```

## Invalid Regex

```text
match.regex: regex failed to compile
```

TrustLoopGuard uses Rust regex syntax. Lookaround is not supported.

## Missing Rewrite

```text
rewrite: rewrite is required when action is rewrite
```

Fix:

```yaml
action: rewrite
rewrite: "I can help review eligibility, but I can't guarantee the outcome."
```

## Empty Matcher

```text
match.literal: literal matcher must not be empty
```

Every matcher must contain useful text.

## Empty Scope Value

```text
when.agents[0]: must not be empty
```

Remove empty list values or replace them with a real scope value.

