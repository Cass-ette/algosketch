string reverse_string(string text) {
    string result = "";
    for (int i = text.size() - 1; i >= 0; i = i - 1) {
        result = result + text[i];
    }
    return result;
}
