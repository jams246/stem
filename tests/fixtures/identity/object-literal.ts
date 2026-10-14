const api = {
  load() {
    return 1;
  },
  save: (x: number) => x,
  remove: function () {
    return 0;
  },
  nested: {
    deep() {
      return 2;
    },
  },
};
