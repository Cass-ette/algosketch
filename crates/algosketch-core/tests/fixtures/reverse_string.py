def reverse_string(text):
    result: str = ""
    for i in range(len(text) - 1, -1, -1):
        result = result + text[i]
    return result
