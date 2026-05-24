int two_sum(int items[], int target, int size) {
    for (int i = 0; i < size; i = i + 1) {
        for (int j = i + 1; j < size; j = j + 1) {
            if (items[i] + items[j] == target) {
                return i;
            }
        }
    }
    return -1;
}
