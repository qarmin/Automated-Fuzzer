class TrainLog(object):
                    pbar += '{}={.3f} '.format(metric_name, metric[0])
                    if len(metric) == 2:
                        pbar += 'valid_{}={:.3f} '.format(metric_name,
                                                          valid_metric)
