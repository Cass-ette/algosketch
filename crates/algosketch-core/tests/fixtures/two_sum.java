class Algorithms {
    int two_sum(int[] items, int target) {
        for (int i = 0; i < items.length; i = i + 1) {
            for (int j = i + 1; j < items.length; j = j + 1) {
                if (items[i] + items[j] == target) {
                    return i;
                }
            }
        }
        return -1;
    }
}
