class Algorithms {
    String reverse_string(String text) {
        String result = "";
        for (int i = text.length() - 1; i >= 0; i = i - 1) {
            result = result + text.charAt(i);
        }
        return result;
    }
}
