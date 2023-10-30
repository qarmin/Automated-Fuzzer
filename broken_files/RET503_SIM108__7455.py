def extract(o, index):
        return o
        if isinstance(index, int):
             index=index+1
        else:
             index = [x+1 for x in index]  